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
use crate::features::agent_bridge::{put_bridge_env, session_ended, write_launch_files};
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
    if write_launch_files(&root, &dir, &injection.files).is_err() {
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
    put_bridge_env(spec, wire::REPORT_TOKEN_ENV, token, &socket, &program);
    spec.args = injection.args;
    state
        .runtime
        .expect_reports(session, Timestamp::now().millis());
    true
}

/// How long a session whose hooks report may go without its first one:
/// a tool reports when its session opens (Claude Code's `SessionStart`),
/// so a session still silent after this waits on the tool's own screen
/// before it, in its terminal (a folder-trust question, a Resume picker
/// that found nothing, a sign-in).
pub const FIRST_REPORT_MS: i64 = 20_000;

/// The id prefix of the ask [`first_report_overdue`] stands in with.
const FIRST_REPORT_ASK: &str = "hook-first-";

/// What a first report overdue says when its tool's screen asks nothing
/// its adapter knows (the UI's rows then say "open it to see").
pub const WAITS_IN_TERMINAL: &str = "It waits in its terminal: answer it there";

/// What a session whose first report is overdue is taken to be: waiting
/// for the person in its terminal, never working (the rc19 walk: Continue
/// in AI Juggler's resume sat in Claude's Resume picker, "No sessions
/// match", and read Working). `question`: what its tool's own screen asks
/// ([`screen_question`]), its words (the rc29 persona walks: the row
/// read "It waits on its own screen…", not "Choose the text style…").
#[must_use]
pub fn first_report_overdue(
    session: &SessionId,
    now_ms: i64,
    question: Option<String>,
) -> Observed {
    Observed {
        state: "asks_you",
        ask: Some(AskInfo {
            id: format!("{FIRST_REPORT_ASK}{now_ms}"),
            session_id: session.to_string(),
            kind: "question".to_owned(),
            approval_kind: None,
            summary: question.unwrap_or_else(|| WAITS_IN_TERMINAL.to_owned()),
            reason: None,
            options: Vec::new(),
            parts: Vec::new(),
            asked_at_ms: now_ms,
        }),
        error: None,
    }
}

/// Takes in every session whose first report is overdue at `now_ms` as
/// waiting for the person: its ask stored, the UI told. The safety net
/// under each session's own timer ([`arm_first_reports`]), on the 15 s
/// heuristic tick.
pub fn overdue_tick(state: &Arc<AppState>, now_ms: i64) {
    let asked = |session: &SessionId| screen_question(state, session);
    for (session, now) in state.runtime.take_in_overdue(now_ms, &asked) {
        crate::features::runtime::driver::store_ask(state, &session, now.ask.as_ref(), false);
    }
}

/// `session`'s first-report timer ran out at `now_ms`: when nothing has
/// reported, the same path as [`overdue_tick`] (its ask stored, Needs you
/// and the UI told).
pub fn first_report_due(state: &Arc<AppState>, session: &SessionId, now_ms: i64) {
    let asked = |session: &SessionId| screen_question(state, session);
    if let Some(now) = state.runtime.take_in_first_report(session, now_ms, &asked) {
        crate::features::runtime::driver::store_ask(state, session, now.ask.as_ref(), false);
    }
}

/// How much of a session's output its tool's screen question is read
/// from.
const SCREEN_TAIL: usize = 8192;

/// What `session`'s tool's own screen asks now, as its adapter reads the
/// end of its terminal (`ToolConfig::screen_ask`); `None` when it asks
/// nothing the adapter knows.
pub fn screen_question(state: &AppState, session: &SessionId) -> Option<String> {
    let harness = {
        let storage = state.storage.lock().ok()?;
        let record = storage.sessions().get(session).ok()?;
        let profile = storage.profiles().get(&record.profile).ok()?;
        storage.accounts().get(&profile.account).ok()?.harness
    };
    let adapter = state.registry.get(&harness)?;
    let tail = state.supervisor.output_tail(session, SCREEN_TAIL)?;
    adapter.screen_ask(&tail)
}

/// Holds a one-shot timer: dropping it cancels the timer.
pub struct Cancel {
    _held: Box<dyn std::any::Any + Send>,
}

impl Cancel {
    /// A guard that cancels its timer when dropped (what it holds goes
    /// with it).
    #[must_use]
    pub fn new(held: impl std::any::Any + Send) -> Self {
        Self {
            _held: Box::new(held),
        }
    }
}

/// Runs something once, later.
pub trait OneShot: Send + Sync {
    /// Runs `due` (with the time it runs, epoch ms) after `wait`, unless
    /// the returned guard is dropped first; `None` when no timer could be
    /// set (the 15 s safety net still takes the session in).
    fn after(&self, wait: std::time::Duration, due: Box<dyn FnOnce(i64) + Send>) -> Option<Cancel>;
}

/// The app's one-shot timer: a thread that waits once and ends, woken
/// early by its guard's drop. One per terminal launch, never a tick.
pub struct ThreadTimer;

impl OneShot for ThreadTimer {
    fn after(&self, wait: std::time::Duration, due: Box<dyn FnOnce(i64) + Send>) -> Option<Cancel> {
        let (guard, cancelled) = std::sync::mpsc::channel::<()>();
        std::thread::Builder::new()
            .name("first-report".to_owned())
            .spawn(move || {
                // One wait, ended by the deadline or by the guard's drop
                // (the first report, the process's end): never repeated.
                if cancelled.recv_timeout(wait) == Err(std::sync::mpsc::RecvTimeoutError::Timeout) {
                    let mut due = Some(due);
                    crate::ticks::once(&mut || {
                        if let Some(due) = due.take() {
                            due(Timestamp::now().millis());
                        }
                    });
                }
            })
            .ok()?;
        Some(Cancel::new(guard))
    }
}

/// [`arm_first_reports`] on the app's own [`ThreadTimer`] (at start).
pub fn arm(state: &Arc<AppState>) {
    arm_first_reports(state, Arc::new(ThreadTimer));
}

/// From here on, each session [`super::Runtimes::expect_reports`] marks
/// gets a one-shot timer on `clock` at [`FIRST_REPORT_MS`]: still silent
/// then, it reads Asks you at once, not on the next tick (the rc22 walk:
/// about 40 s for a folder-trust question).
pub fn arm_first_reports(state: &Arc<AppState>, clock: Arc<dyn OneShot>) {
    let weak = Arc::downgrade(state);
    state
        .runtime
        .set_first_report_timer(Arc::new(move |session: &SessionId| {
            let weak = weak.clone();
            let session = session.clone();
            clock.after(
                std::time::Duration::from_millis(FIRST_REPORT_MS.unsigned_abs()),
                Box::new(move |now_ms| {
                    if let Some(state) = weak.upgrade() {
                        first_report_due(&state, &session, now_ms);
                    }
                }),
            )
        }));
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

impl Observed {
    /// Whether this is [`first_report_overdue`]'s stand-in, not anything
    /// the tool said.
    #[must_use]
    pub fn is_first_report_overdue(&self) -> bool {
        self.ask
            .as_ref()
            .is_some_and(|ask| ask.id.starts_with(FIRST_REPORT_ASK))
    }
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
    // The rc20 walk: any report means the tool's session has opened, so
    // the overdue stand-in is never carried on (a report that keeps the
    // state before, or the ask before, starts from nothing instead).
    let before = before.filter(|before| !before.is_first_report_overdue());
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

/// What a tool's finished step started, in the Processes page's words
/// ("Bash: npm run dev"): only a `PostToolUse` that names its tool and
/// what it ran.
fn started_label(hook: &HookReport) -> Option<String> {
    if hook.event != "PostToolUse" {
        return None;
    }
    Some(format!(
        "{}: {}",
        hook.tool.as_ref()?,
        hook.summary.as_ref()?
    ))
}

/// One report, answered.
#[must_use]
pub fn handle(state: &Arc<AppState>, report: &Report) -> Response {
    if report.v != wire::VERSION || !wire::HOOK_EVENTS.contains(&report.hook.event.as_str()) {
        logged(None, report, Some("unreadable"));
        return Response::refused(Refusal::Bad, "AI Juggler couldn't read that report.");
    }
    let Some(session) = state
        .agent_bridge
        .session_of_reports(&state.data_dir, &report.token)
        .filter(|session| !session_ended(state, session.as_str()))
    else {
        logged(None, report, Some("key not current"));
        return Response::refused(Refusal::Unknown, "This session's report key isn't current.");
    };
    logged(Some(&session), report, None);
    // AJ-E2401: what a command the tool ran started is named on the
    // Processes page ("Started by Bash: npm run dev"), from memory only.
    if let Some(label) = started_label(&report.hook) {
        state
            .processes
            .note_label(session.as_str(), &label, Timestamp::now().millis());
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

/// Debug mode's log keeps the reports (rc7 walk: reports arrived, the
/// session's "Runs in" changed, and the log had no line of them): the
/// event, the tool's name, and why one was refused. Never what was asked
/// or run (the summary), nor the key. At most one line per session (or
/// refusal) every [`REPORT_LINE_MS`], the next saying how many it
/// skipped, so a busy session can't flood the log.
fn logged(session: Option<&SessionId>, report: &Report, refused: Option<&'static str>) {
    let key = session.map_or_else(
        || format!("refused:{}", refused.unwrap_or("-")),
        |s| s.as_str().to_owned(),
    );
    let Some(skipped) = report_line_due(&key, Timestamp::now().millis()) else {
        return;
    };
    let step: String = report.hook.event.chars().take(32).collect();
    let tool: String = report
        .hook
        .tool
        .as_deref()
        .unwrap_or("-")
        .chars()
        .take(40)
        .collect();
    let session = session.map_or("-", SessionId::as_str);
    if let Some(why) = refused {
        tracing::info!(
            target: crate::features::app::diagnostics::debug_log::TARGET,
            flow = "report",
            step = step.as_str(),
            tool = tool.as_str(),
            refused = why,
            skipped,
            "agent report"
        );
    } else {
        tracing::info!(
            target: crate::features::app::diagnostics::debug_log::TARGET,
            flow = "report",
            step = step.as_str(),
            session,
            tool = tool.as_str(),
            skipped,
            "agent report"
        );
    }
}

/// The least time between two report lines of one session.
const REPORT_LINE_MS: i64 = 5_000;
/// The most sessions remembered for [`report_line_due`].
const REPORT_LINE_KEYS: usize = 256;

/// The reports skipped since `key`'s last line when one is due at
/// `now_ms`, else `None` (the skip is counted).
fn report_line_due(key: &str, now_ms: i64) -> Option<u32> {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    static LAST: OnceLock<Mutex<HashMap<String, (i64, u32)>>> = OnceLock::new();
    let mut last = LAST.get_or_init(Mutex::default).lock().ok()?;
    if let Some((at, skipped)) = last.get_mut(key) {
        if now_ms - *at < REPORT_LINE_MS {
            *skipped = skipped.saturating_add(1);
            return None;
        }
        let was = *skipped;
        *at = now_ms;
        *skipped = 0;
        return Some(was);
    }
    if last.len() >= REPORT_LINE_KEYS {
        last.retain(|_, (at, _)| now_ms - *at < REPORT_LINE_MS);
        if last.len() >= REPORT_LINE_KEYS {
            last.clear();
        }
    }
    last.insert(key.to_owned(), (now_ms, 0));
    Some(0)
}

#[cfg(test)]
mod report_line_tests {
    use super::{REPORT_LINE_MS, report_line_due};

    #[test]
    fn a_busy_session_gets_one_line_per_spell_and_the_next_counts_the_rest() {
        let key = "report-line-test-session";
        assert_eq!(report_line_due(key, 1_000), Some(0));
        assert_eq!(report_line_due(key, 1_100), None);
        assert_eq!(report_line_due(key, 2_000), None);
        assert_eq!(report_line_due(key, 1_000 + REPORT_LINE_MS), Some(2));
        assert_eq!(
            report_line_due("report-line-test-other", 1_100),
            Some(0),
            "per session"
        );
    }
}

/// AJ-E2401 R4: the hook's finished step names what it started.
#[cfg(test)]
mod label_tests {
    use super::started_label;
    use juggler_agentbridge::wire::HookReport;

    fn hook(event: &str, tool: Option<&str>, summary: Option<&str>) -> HookReport {
        HookReport {
            event: event.to_owned(),
            tool: tool.map(str::to_owned),
            summary: summary.map(str::to_owned),
            ..HookReport::default()
        }
    }

    #[test]
    fn a_finished_step_names_its_tool_and_what_it_ran_and_nothing_else_does() {
        assert_eq!(
            started_label(&hook("PostToolUse", Some("Bash"), Some("npm run dev"))).as_deref(),
            Some("Bash: npm run dev")
        );
        // Asked for, not done yet: nothing started.
        assert_eq!(
            started_label(&hook("PreToolUse", Some("Bash"), Some("npm run dev"))),
            None
        );
        // Without its tool or what it ran, no guess.
        assert_eq!(
            started_label(&hook("PostToolUse", None, Some("npm run dev"))),
            None
        );
        assert_eq!(
            started_label(&hook("PostToolUse", Some("Bash"), None)),
            None
        );
    }
}
