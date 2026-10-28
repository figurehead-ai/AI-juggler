//! AJ-E1102: usage statistics in the app (D-19, ADR-0015).
//!
//! The choice is stored per Mac as `usage_stats`
//! (`{mode, decided_at_ms, consent_version}`; never in a backup). At
//! launch the overrides are applied and, unless the result is off, the
//! recorder starts. Turning stats off deletes the whole `stats/` folder.
//! Built without the `stats` feature, the crate isn't in the binary and
//! every answer says "not included in this build".

// Tauri command signatures take State/String by value (as in commands.rs).
#![allow(clippy::needless_pass_by_value)]

use std::path::PathBuf;
use std::sync::Arc;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

// The commands (each off the main thread) are their own module.
mod commands;
pub use commands::*;

/// The setting key.
pub const SETTING_KEY: &str = "usage_stats";
/// The consent text's version (bump it to ask again — AJ-E1105).
pub const CONSENT_VERSION: u32 = 1;

/// The stored choice.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UsageStatsChoice {
    /// `off` | `local` | `share`.
    pub mode: String,
    /// When the user chose, epoch ms (`None`: never asked).
    pub decided_at_ms: Option<i64>,
    /// The consent version they answered.
    pub consent_version: u32,
}

/// What Settings ▸ Privacy shows.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct UsageStatsInfo {
    /// This build includes usage statistics.
    pub included: bool,
    /// The user's choice: `off` | `local` | `share`.
    pub chosen: String,
    /// What applies after the overrides.
    pub effective: String,
    /// Reports may be uploaded.
    pub can_upload: bool,
    /// Why `effective` differs from `chosen`, in plain words.
    pub reason: Option<String>,
    /// When the user chose, epoch ms.
    #[ts(type = "number | null")]
    pub decided_at_ms: Option<i64>,
    /// The consent version they answered (0: never asked).
    pub consent_version: u32,
    /// The current consent version.
    pub current_consent_version: u32,
}

/// The result of changing the mode.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct UsageStatsSetResult {
    /// The new state.
    pub info: UsageStatsInfo,
    /// What happened, in plain words.
    pub message: String,
}

/// The app's `stats/` folder.
#[must_use]
pub fn stats_dir(state: &AppState) -> PathBuf {
    state.data_dir.join("stats")
}

/// The stored choice (off when unset or unreadable).
#[must_use]
pub fn choice(state: &Arc<AppState>) -> UsageStatsChoice {
    let stored = state
        .storage
        .lock()
        .ok()
        .and_then(|storage| storage.app_settings().get(SETTING_KEY).ok().flatten())
        .and_then(|raw| serde_json::from_str::<UsageStatsChoice>(&raw).ok());
    match stored {
        Some(choice) if matches!(choice.mode.as_str(), "off" | "local" | "share") => choice,
        _ => UsageStatsChoice {
            mode: "off".to_owned(),
            ..UsageStatsChoice::default()
        },
    }
}

fn save(state: &Arc<AppState>, choice: &UsageStatsChoice) -> AppResult<()> {
    let raw = serde_json::to_string(choice).map_err(|e| AppError::Io {
        context: "usage stats",
        message: e.to_string(),
    })?;
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    storage.app_settings().set(SETTING_KEY, &raw)?;
    Ok(())
}

// ------------------------------------------------- AJ-E1103: emitting

/// Counts one `metric` event with these wire dimensions. Anything the
/// schema doesn't list exactly is dropped (and trips a debug assertion),
/// so an emitting site can never widen what is collected. A no-op while
/// stats are off, and in builds without the `stats` feature.
pub fn count(metric: &'static str, dims: &[(&'static str, &str)]) {
    count_n(metric, dims, 1);
}

/// Counts `metric` `n` times (a total, e.g. tokens).
pub fn count_n(metric: &'static str, dims: &[(&'static str, &str)], n: u64) {
    #[cfg(feature = "stats")]
    if n > 0 && recording() {
        let parsed = juggler_stats::StatEvent::parse(metric, dims, None);
        debug_assert!(parsed.is_some(), "not in the schema: {metric} {dims:?}");
        if let Some(event) = parsed {
            juggler_stats::record_n(event, n);
        }
    }
    #[cfg(not(feature = "stats"))]
    let _ = (metric, dims, n);
}

/// Observes `value` for a histogram `metric`.
pub fn observe(metric: &'static str, dims: &[(&'static str, &str)], value: f64) {
    #[cfg(feature = "stats")]
    if juggler_stats::recording() {
        let parsed = juggler_stats::StatEvent::parse(metric, dims, Some(value));
        debug_assert!(parsed.is_some(), "not in the schema: {metric} {dims:?}");
        if let Some(event) = parsed {
            juggler_stats::record(event);
        }
    }
    #[cfg(not(feature = "stats"))]
    let _ = (metric, dims, value);
}

/// A Provider id as the schema names it (`other` for anything unlisted).
#[must_use]
pub fn provider(harness: &str) -> &'static str {
    match harness {
        "claude-code" => "claude-code",
        "codex" => "codex",
        "zcode" => "zcode",
        _ => "other",
    }
}

/// A model name reduced to its family (`other` when unknown). The name
/// itself is never counted.
#[must_use]
pub fn model_family(model: &str) -> &'static str {
    let model = model.to_ascii_lowercase();
    let has = |needle: &str| model.contains(needle);
    if has("opus") {
        "opus"
    } else if has("sonnet") {
        "sonnet"
    } else if has("haiku") {
        "haiku"
    } else if has("gpt-5-codex") || has("gpt-5.1-codex") {
        "gpt-5-codex"
    } else if has("gpt-5-mini") {
        "gpt-5-mini"
    } else if has("gpt-5") {
        "gpt-5"
    } else if has("glm-5") {
        "glm-5"
    } else if has("glm-4") {
        "glm-4"
    } else {
        "other"
    }
}

/// An Instance started: `origin` is `new`, `resume`, `switch` or
/// `relaunch` (an undo brings the source back).
pub(crate) fn instance_started(harness: &str, desktop: bool, origin: &'static str) {
    count(
        "instance.started",
        &[
            ("provider", provider(harness)),
            ("kind", if desktop { "desktop" } else { "terminal" }),
            ("origin", origin),
        ],
    );
}

/// An Instance exited (`outcome`: finished, stopped, failed, crashed):
/// counts it, its minutes, and a quota block.
pub(crate) fn instance_ended(
    storage: &juggler_core::storage::Storage,
    record: &juggler_core::model::InstanceRecord,
    outcome: &'static str,
) {
    if !recording() {
        return;
    }
    let harness = harness_of_recipe(storage, &record.profile);
    let provider = provider(&harness);
    count(
        "instance.ended",
        &[("provider", provider), ("outcome", outcome)],
    );
    let now = juggler_core::model::Timestamp::now().millis();
    let minutes = (now - record.started_at.millis()).max(0) / 60_000;
    count_n(
        "instance.minutes",
        &[("provider", provider)],
        u64::try_from(minutes).unwrap_or(0),
    );
    if record.status == juggler_core::model::SessionStatus::QuotaBlocked {
        count(
            "instance.state_entered",
            &[("provider", provider), ("state", "quota_blocked")],
        );
    }
}

/// An Instance entered waiting-for-you or stuck.
pub(crate) fn instance_state_entered(
    storage: &juggler_core::storage::Storage,
    record: &juggler_core::model::InstanceRecord,
) {
    use juggler_core::model::SessionStatus;
    let state = match record.status {
        SessionStatus::WaitingUser => "waiting_user",
        SessionStatus::Stuck => "stuck",
        _ => return,
    };
    if recording() {
        let harness = harness_of_recipe(storage, &record.profile);
        count(
            "instance.state_entered",
            &[("provider", provider(&harness)), ("state", state)],
        );
    }
}

/// Token totals of newly stored usage rows, Measured only (D-06):
/// estimates and computed values are never counted.
pub(crate) fn tokens_ingested(
    accounts: &[juggler_core::model::Account],
    events: &[juggler_core::quota::UsageEvent],
    stored: &[juggler_core::storage::Stored],
) {
    if !recording() {
        return;
    }
    let mut totals: std::collections::BTreeMap<(&'static str, &'static str), u64> =
        std::collections::BTreeMap::new();
    for (event, stored) in events.iter().zip(stored) {
        if event.confidence != juggler_core::model::Confidence::Measured {
            continue;
        }
        let harness = accounts
            .iter()
            .find(|a| a.id.as_str() == event.account)
            .map(|a| a.harness.as_str())
            .unwrap_or_default();
        let provider = provider(harness);
        let tokens = &stored.added;
        for (direction, n) in [
            ("input", tokens.input),
            ("output", tokens.output.saturating_add(tokens.reasoning)),
            (
                "cache",
                tokens.cache_read.saturating_add(tokens.cache_write),
            ),
        ] {
            let slot = totals.entry((provider, direction)).or_default();
            *slot = slot.saturating_add(n);
        }
    }
    for ((provider, direction), n) in totals {
        count_n(
            "session.tokens",
            &[("provider", provider), ("direction", direction)],
            n,
        );
    }
}

/// The 5 s resource sample: the week's peak of concurrent Instances (a
/// gauge, raised as it grows) and, once an hour, each live Instance's
/// helper count.
pub(crate) fn resources_sampled(live: &[crate::commands::InstanceResourcesInfo]) {
    #[cfg(feature = "stats")]
    {
        use std::sync::Mutex;
        /// (week, peak so far, last helper-count sample, epoch ms).
        static SEEN: Mutex<(String, u64, i64)> = Mutex::new((String::new(), 0, 0));
        if !recording() {
            return;
        }
        let now = juggler_core::model::Timestamp::now().millis();
        let week = juggler_stats::week::label(now);
        let mut seen = SEEN
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if seen.0 != week {
            *seen = (week, 0, seen.2);
        }
        let concurrent = u64::try_from(live.len()).unwrap_or(u64::MAX);
        if concurrent > seen.1 {
            seen.1 = concurrent;
            juggler_stats::set_gauge(juggler_stats::StatGauge::InstancePeakConcurrent {
                count: concurrent,
            });
        }
        if now - seen.2 >= 3_600_000 {
            seen.2 = now;
            for instance in live {
                let children = f64::from(instance.child_count.unwrap_or(0));
                observe("process.children_per_instance", &[], children);
            }
        }
    }
    #[cfg(not(feature = "stats"))]
    let _ = live;
}

/// One switch being timed: started, then exactly one of succeeded /
/// failed / rolled back.
pub(crate) struct Switch {
    kind: &'static str,
    from: &'static str,
    to: &'static str,
    mechanism: &'static str,
    trigger: &'static str,
    started: std::time::Instant,
}

impl Switch {
    /// Counts `switch.started`. `mechanism`: `native_resume`,
    /// `transplant` or `pack_replay`; `trigger`: `quota_wall`,
    /// `suggestion`, `manual` or `remote`.
    pub(crate) fn started(
        from_harness: &str,
        to_harness: &str,
        mechanism: &'static str,
        trigger: &'static str,
    ) -> Self {
        let kind = if from_harness == to_harness {
            "account"
        } else {
            "provider"
        };
        count("switch.started", &[("kind", kind), ("trigger", trigger)]);
        Self {
            kind,
            from: provider(from_harness),
            to: provider(to_harness),
            mechanism,
            trigger,
            started: std::time::Instant::now(),
        }
    }

    fn completed(&self, outcome: &'static str) {
        self.count_completed(outcome);
        observe(
            "switch.duration_s",
            &[("kind", self.kind)],
            self.started.elapsed().as_secs_f64(),
        );
    }

    fn count_completed(&self, outcome: &'static str) {
        count(
            "switch.completed",
            &[
                ("kind", self.kind),
                ("from", self.from),
                ("to", self.to),
                ("mechanism", self.mechanism),
                ("trigger", self.trigger),
                ("outcome", outcome),
            ],
        );
    }

    /// The new Instance runs.
    pub(crate) fn succeeded(&self) {
        self.completed("success");
    }

    /// The pipeline stopped at `stage` (the source is untouched).
    pub(crate) fn failed(&self, stage: juggler_core::model::FailureStage) {
        use juggler_core::model::FailureStage;
        let stage = match stage {
            FailureStage::Resolve | FailureStage::Preflight | FailureStage::Freeze => "freeze",
            FailureStage::Snapshot | FailureStage::Stage => "stage",
            FailureStage::Verify => "verify",
            FailureStage::Launch | FailureStage::Confirm => "launch",
        };
        count(
            "switch.failed_stage",
            &[("kind", self.kind), ("stage", stage)],
        );
        self.completed("failed");
    }
}

/// The user undid a completed switch (the source resumed): counted as a
/// `rolled_back` completion of that edge, untimed.
pub(crate) fn switch_undone(edge: &juggler_core::model::SwitchEdge) {
    let (from, to) = (edge.from_provider.as_str(), edge.to_provider.as_str());
    Switch {
        kind: if from == to { "account" } else { "provider" },
        from: provider(from),
        to: provider(to),
        mechanism: mechanism(edge.mechanism),
        trigger: "manual",
        started: std::time::Instant::now(),
    }
    .count_completed("rolled_back");
}

/// The schema's switch mechanism for an edge's.
pub(crate) const fn mechanism(mechanism: juggler_core::model::SwitchMechanism) -> &'static str {
    use juggler_core::model::SwitchMechanism;
    match mechanism {
        SwitchMechanism::NativeResume => "native_resume",
        SwitchMechanism::Transplant => "transplant",
        SwitchMechanism::Replay | SwitchMechanism::Pack => "pack_replay",
    }
}

/// The bytes under `dir` (a pack's size; never its contents or names).
pub(crate) fn dir_bytes(dir: &std::path::Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| match entry.file_type() {
            Ok(kind) if kind.is_dir() => dir_bytes(&entry.path()),
            Ok(_) => entry.metadata().map_or(0, |m| m.len()),
            Err(_) => 0,
        })
        .sum()
}

/// The schema's leftover category for a process (from its role guess;
/// the name and command line themselves are never counted).
#[must_use]
pub fn leftover_category(exe: &str, cmdline: &str) -> &'static str {
    let hay = format!("{exe} {cmdline}").to_ascii_lowercase();
    if [
        "chrome",
        "chromium",
        "firefox",
        "playwright",
        "puppeteer",
        "webkit",
    ]
    .iter()
    .any(|needle| hay.contains(needle))
    {
        return "browser";
    }
    match crate::commands::guess_role(exe, cmdline) {
        "DevServer" => "dev_server",
        "McpServer" => "mcp",
        "Watcher" => "watcher",
        _ => "other",
    }
}

/// A usage alert fired (`kind`: `tray-amber`, `actionable` or
/// `exhausted`) for a window (`five_hour` or `weekly`).
pub(crate) fn quota_alert(harness: &str, slot: &str, kind: &str) {
    let provider = provider(harness);
    match kind {
        "tray-amber" => count(
            "usage.threshold_alert",
            &[("provider", provider), ("level", "warn")],
        ),
        "actionable" => count(
            "usage.threshold_alert",
            &[("provider", provider), ("level", "critical")],
        ),
        _ => count(
            "usage.wall_hit",
            &[
                ("provider", provider),
                ("window", if slot == "five_hour" { "5h" } else { "weekly" }),
            ],
        ),
    }
}

/// The memory these processes hold now, bytes (0 while stats are off).
pub(crate) fn rss_of(state: &AppState, pids: &[u32]) -> u64 {
    if !recording() {
        return 0;
    }
    state.resources.lock().map_or(0, |collector| {
        pids.iter()
            .filter_map(|pid| collector.lookup(*pid))
            .map(|(_, rss)| rss)
            .sum()
    })
}

/// A verified kill succeeded: the leftover cleaned, by hand or by an
/// automatic tier, with the ports and memory it freed.
pub(crate) fn leftover_killed(
    row: &juggler_core::storage::LedgerRow,
    actor: &str,
    ports_before: Option<&str>,
    still_held: &[u16],
    rss_before: u64,
) {
    let ports_freed = ports_before
        .unwrap_or_default()
        .split(',')
        .filter_map(|port| port.trim().parse::<u16>().ok())
        .filter(|port| !still_held.contains(port))
        .count();
    leftover_cleaned(
        &row.exe_path,
        &row.cmdline_trunc,
        actor.starts_with("auto:"),
        u64::try_from(ports_freed).unwrap_or(0),
        rss_before,
    );
}

/// A leftover was stopped: its category, by hand or automatically, and
/// what that freed.
pub(crate) fn leftover_cleaned(
    exe: &str,
    cmdline: &str,
    auto: bool,
    ports_freed: u64,
    rss_freed_bytes: u64,
) {
    if !recording() {
        return;
    }
    count(
        "leftover.cleaned",
        &[
            ("category", leftover_category(exe, cmdline)),
            ("mode", if auto { "auto" } else { "manual" }),
        ],
    );
    count_n("cleanup.ports_freed", &[], ports_freed);
    count_n(
        "cleanup.memory_freed_mb",
        &[],
        rss_freed_bytes / (1024 * 1024),
    );
}

/// A resource alert was shown (`kind`: `cpu_hog`, `memory_hog` or
/// `idle`): counted once per process per launch, however often the list
/// is polled.
pub(crate) fn resource_alert_shown(kind: &'static str, session: &str, pid: u32) {
    use std::sync::Mutex;
    /// (session, pid, kind) already counted this launch.
    type Seen = std::collections::HashSet<(String, u32, &'static str)>;
    static SHOWN: Mutex<Option<Seen>> = Mutex::new(None);
    if !recording() {
        return;
    }
    let mut shown = SHOWN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if shown
        .get_or_insert_with(std::collections::HashSet::new)
        .insert((session.to_owned(), pid, kind))
    {
        count("resource.alert", &[("kind", kind), ("action", "shown")]);
    }
}

/// A pairing attempt ended: `None` paired, otherwise the link's stable
/// reason code (mapped to the schema's; anything else is `other`).
pub(crate) fn fleet_pairing(reason_code: Option<&str>) {
    let (outcome, reason) = match reason_code {
        None => ("completed", "none"),
        Some("expired") => ("expired", "code_expired"),
        Some(code) => (
            "failed",
            match code {
                "mismatch" | "not_found" | "not_open" => "code_invalid",
                "unreachable" => "network",
                "fingerprint" => "tls",
                "version" => "version",
                "cancelled" | "locked_out" | "disabled" | "denied" => "denied",
                _ => "other",
            },
        ),
    };
    count("fleet.pairing", &[("outcome", outcome), ("reason", reason)]);
}

/// The schema's `(area, code)` for an error, from its variant and the
/// static context the code gave it — never its message. `None` for
/// refusals that aren't failures (validation, not found, an Account
/// signed in as someone else, already imported).
#[must_use]
pub fn error_class(error: &crate::error::AppError) -> Option<(&'static str, &'static str)> {
    use crate::error::AppError;
    use juggler_adapter::AdapterError;
    use juggler_supervisor::SupervisorError;
    Some(match error {
        AppError::Core(core) => ("storage", core.stats_code()?),
        AppError::AuditUnavailable => ("storage", "io_error"),
        AppError::Supervisor(e) => (
            "supervisor",
            match e {
                SupervisorError::Spawn { .. } => "spawn_failed",
                SupervisorError::NotATty { .. } => "pty_failed",
                SupervisorError::UnknownSession(_) => "not_found",
                SupervisorError::WrongMachine { .. } | SupervisorError::Unverifiable { .. } => {
                    "invalid_state"
                }
                SupervisorError::LedgerDegraded => "io_error",
                _ => "other",
            },
        ),
        AppError::Adapter(e) => (
            "adapter",
            match e {
                AdapterError::NotFound(_) => "not_found",
                AdapterError::VersionDrift { .. } | AdapterError::Unsupported(_) => {
                    "version_unsupported"
                }
                AdapterError::InvalidLaunch(_) => "invalid_state",
                AdapterError::AccountDir { .. } | AdapterError::Io(_) | AdapterError::Watch(_) => {
                    "io_error"
                }
                _ => "other",
            },
        ),
        AppError::UnknownHarness(_) => ("adapter", "not_found"),
        AppError::Io { context, .. } => (
            match *context {
                "hand over" | "undo switch" | "switch back" | "export pack" | "resume epoch"
                | "redaction policy" | "graph action" | "parked instances" => "switch",
                "Fleet discovery"
                | "start the Fleet link"
                | "remote action"
                | "Fleet link"
                | "fleet command sequence"
                | "fleet" => "fleet",
                "update" => "updater",
                "worktrees" | "create the worktree" | "remove the worktree" => "supervisor",
                _ => "ui",
            },
            "io_error",
        ),
        AppError::StatePoisoned => ("ui", "invalid_state"),
        AppError::Switch { failure, .. } => ("switch", switch_code(failure)),
        AppError::Validation { .. }
        | AppError::NotFound { .. }
        | AppError::IdentityMismatch { .. }
        | AppError::AlreadyImported { .. }
        | AppError::Refused(_) => return None,
    })
}

/// A switch failure's class (its kind, never its text).
fn switch_code(failure: &juggler_handover::pipeline::SwitchFailure) -> &'static str {
    use juggler_handover::pipeline::{PreflightStop, SwitchFailure};
    match failure {
        SwitchFailure::LaunchFailed(_) => "spawn_failed",
        SwitchFailure::StagingAborted(_) | SwitchFailure::Adapter(_) => "io_error",
        SwitchFailure::UnresolvedSource
        | SwitchFailure::TornTail
        | SwitchFailure::CollisionForked(_)
        | SwitchFailure::MergeConflictOpen
        | SwitchFailure::OriginalSessionGone => "invalid_state",
        SwitchFailure::Preflight(PreflightStop::VersionDrift) => "version_unsupported",
        _ => "other",
    }
}

/// An error reached the interface (counted by class only).
pub(crate) fn error_surfaced(error: &crate::error::AppError) {
    if recording()
        && let Some((area, code)) = error_class(error)
    {
        count("error.class", &[("area", area), ("code", code)]);
    }
}

/// An onboarding scan finished: what it found, by Provider and how
/// sure it is, and where installs came from (never a path or name).
pub(crate) fn scan_completed(
    installs: &[crate::commands::DiscoveredInfo],
    managers: &[juggler_adapter::environment::ManagerHint],
) {
    if !recording() {
        return;
    }
    count("onboarding.scan_completed", &[]);
    for install in installs {
        let provenance = match install.inference.purpose_confidence.as_str() {
            "found" => "found",
            "likely" => "likely",
            _ => "guessed",
        };
        count(
            "onboarding.found",
            &[
                ("provider", provider(&install.harness)),
                ("provenance", provenance),
            ],
        );
        // A config dir found by its name isn't a schema source: not counted.
        let source = match install.origin.as_str() {
            "vendor-default" => Some("vendor_default"),
            "env-override" => Some("env_override"),
            "shell-alias" => Some("alias"),
            "other-manager" => Some("other_manager"), // AJ-E1519: a manager's profile
            _ => None,
        };
        if let Some(source) = source {
            count("onboarding.source_detected", &[("source", source)]);
        }
    }
    for manager in managers.iter().filter(|m| m.present) {
        let source = match manager.manager.as_str() {
            "cc-switch" => "cc_switch",
            "t3" => "t3code",
            _ => "other_manager",
        };
        count("onboarding.source_detected", &[("source", source)]);
    }
}

/// An Account import finished; a failure's reason comes from the error's
/// variant only. Importing one that was already imported isn't counted.
pub(crate) fn import_finished(harness: &str, error: Option<&crate::error::AppError>) {
    use crate::error::AppError;
    use juggler_adapter::AdapterError;
    let (outcome, reason) = match error {
        None => ("completed", "none"),
        Some(AppError::AlreadyImported { .. }) => return,
        Some(error) => (
            "failed",
            match error {
                AppError::UnknownHarness(_)
                | AppError::Adapter(
                    AdapterError::Unsupported(_) | AdapterError::VersionDrift { .. },
                ) => "unsupported_layout",
                AppError::Adapter(AdapterError::NotFound(_) | AdapterError::AccountDir { .. })
                | AppError::NotFound { .. } => "source_missing",
                AppError::Io { .. } | AppError::Adapter(AdapterError::Io(_)) => "copy_failed",
                AppError::IdentityMismatch { .. } => "verify_failed",
                _ => "other",
            },
        ),
    };
    count(
        "onboarding.import",
        &[
            ("provider", provider(harness)),
            ("outcome", outcome),
            ("reason_code", reason),
        ],
    );
}

/// One recorded onboarding funnel event, as usage statistics count it.
/// Scans and imports are counted where they happen; logins come from
/// the interface, which knows the Provider.
pub(crate) fn funnel_event(kind: &str, fields: &std::collections::BTreeMap<String, String>) {
    let field = |key: &str| fields.get(key).map(String::as_str);
    match kind {
        "proposal_shown" => count("onboarding.proposal", &[("action", "shown")]),
        "proposal_accepted" => count("onboarding.proposal", &[("action", "accepted")]),
        "proposal_customised" => count("onboarding.proposal", &[("action", "customised")]),
        "expert_opened" => count("onboarding.expert_opened", &[]),
        "first_launch" => {
            if let Some(seconds) =
                field("seconds_since_install").and_then(|s| s.parse::<u32>().ok())
            {
                observe("onboarding.first_launch_s", &[], f64::from(seconds));
            }
        }
        "flow_step" => {
            // Each flow counts once, at the step it opens with.
            let flow = match (field("flow"), field("step")) {
                (Some("a-first-install"), Some("gate" | "scan")) => "first_install",
                (Some("b-new-project"), Some("open")) => "new_project",
                (Some("c-add-provider"), Some("open")) => "add_provider",
                _ => return,
            };
            count("onboarding.flow", &[("flow", flow)]);
        }
        _ => {}
    }
}

/// Stats are being recorded: sites that must look something up first
/// check this, so `off` costs nothing more.
#[must_use]
pub fn recording() -> bool {
    #[cfg(feature = "stats")]
    {
        juggler_stats::recording()
    }
    #[cfg(not(feature = "stats"))]
    {
        false
    }
}

/// The Provider id behind a launch recipe (empty when it can't be read,
/// or while stats are off).
pub(crate) fn harness_of_recipe(
    storage: &juggler_core::storage::Storage,
    recipe: &juggler_core::model::LaunchRecipeId,
) -> String {
    if !recording() {
        return String::new();
    }
    storage
        .profiles()
        .get(recipe)
        .and_then(|profile| storage.accounts().get(&profile.account))
        .map(|account| account.harness.to_string())
        .unwrap_or_default()
}

/// The parts that exist only in builds with the `stats` feature.
#[cfg(feature = "stats")]
pub mod enabled {
    use super::{AppError, AppResult, AppState, Arc, UsageStatsChoice, UsageStatsInfo};
    use juggler_stats::cycle::{Context, Endpoints, Response, Transport};
    use juggler_stats::report::{Arch, Channel, Envelope, OsName};
    use juggler_stats::{Environment, Mode, effective};

    /// The stored mode string as a [`Mode`] (unknown → off).
    #[must_use]
    pub fn mode_of(raw: &str) -> Mode {
        match raw {
            "local" => Mode::Local,
            "share" => Mode::Share,
            _ => Mode::Off,
        }
    }

    /// What Settings shows for `choice` under `env`.
    #[must_use]
    pub fn info(choice: &UsageStatsChoice, env: &Environment) -> UsageStatsInfo {
        let applied = effective(mode_of(&choice.mode), env);
        UsageStatsInfo {
            included: true,
            chosen: choice.mode.clone(),
            effective: applied.mode.wire().to_owned(),
            can_upload: applied.can_upload,
            reason: applied.reason.map(|r| r.words().to_owned()),
            decided_at_ms: choice.decided_at_ms,
            consent_version: choice.consent_version,
            current_consent_version: super::CONSENT_VERSION,
        }
    }

    fn io(e: impl std::fmt::Display) -> AppError {
        AppError::Io {
            context: "usage stats",
            message: e.to_string(),
        }
    }

    /// Starts (or stops) the recorder for the current choice, and notes
    /// the mode at once (a gap in sharing makes that week partial).
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Io`] when the store can't be opened.
    pub fn apply(state: &Arc<AppState>, env: &Environment) -> AppResult<()> {
        let applied = effective(mode_of(&super::choice(state).mode), env);
        let dir = super::stats_dir(state);
        if applied.mode == Mode::Off {
            juggler_stats::shutdown();
            return Ok(());
        }
        juggler_stats::install(juggler_stats::Stats::start(&dir, applied).map_err(io)?);
        juggler_stats::note_mode(&dir, applied).map_err(io)
    }

    /// What every report says about the app and the OS.
    #[must_use]
    pub fn envelope() -> Envelope {
        Envelope {
            app_version: env!("CARGO_PKG_VERSION").to_owned(),
            // The schema (shared with the server) knows beta and stable;
            // nightly is pre-release, so it reports as beta.
            channel: if crate::updates::current_channel() == crate::updates::Channel::Stable {
                Channel::Stable
            } else {
                Channel::Beta
            },
            os: if cfg!(target_os = "macos") {
                OsName::Macos
            } else if cfg!(target_os = "windows") {
                OsName::Windows
            } else {
                OsName::Linux
            },
            os_major: sysinfo::System::os_version()
                .and_then(|v| v.split('.').next()?.trim().parse().ok())
                .unwrap_or(0),
            arch: if std::env::consts::ARCH == "aarch64" {
                Arch::Arm64
            } else {
                Arch::X86_64
            },
        }
    }

    /// The two stats requests over the account client's transport:
    /// `User-Agent: AIJuggler/<version>`, the platform trust store, no
    /// cookies, no redirects, 10 s timeout.
    pub struct StatsTransport(pub Arc<dyn juggler_account::Transport>);

    impl StatsTransport {
        /// The production transport.
        #[must_use]
        pub fn production() -> Self {
            Self(Arc::new(juggler_account::UreqTransport::with_timeout(
                env!("CARGO_PKG_VERSION"),
                std::time::Duration::from_secs(10),
            )))
        }

        fn send(&self, request: &juggler_account::HttpRequest) -> Result<Response, String> {
            self.0
                .send(request)
                .map(|r| Response {
                    status: r.status,
                    retry_after: r.header("retry-after").map(str::to_owned),
                    body: r.body,
                })
                .map_err(|e| e.0)
        }
    }

    impl Transport for StatsTransport {
        fn get(&self, url: &str) -> Result<Response, String> {
            self.send(&juggler_account::HttpRequest {
                method: "GET",
                url: url.to_owned(),
                bearer: None,
                content_type: "application/json",
                headers: Vec::new(),
                body: Vec::new(),
                long_transfer: false,
            })
        }

        fn post_gzip_json(
            &self,
            url: &str,
            body: &[u8],
            bearer: Option<&str>,
        ) -> Result<Response, String> {
            self.send(&juggler_account::HttpRequest {
                method: "POST",
                url: url.to_owned(),
                bearer: bearer.map(str::to_owned),
                content_type: "application/json",
                headers: vec![("content-encoding", "gzip".to_owned())],
                body: body.to_vec(),
                long_transfer: false,
            })
        }
    }

    /// Accounts and launch settings per Provider.
    fn account_gauges(
        storage: &juggler_core::storage::Storage,
        accounts: &[juggler_core::model::Account],
        out: &mut Vec<juggler_stats::StatGauge>,
    ) {
        use juggler_stats::{Provider, StatGauge};
        let mut by_provider: std::collections::BTreeMap<&'static str, (u64, u64)> =
            std::collections::BTreeMap::new();
        for account in accounts {
            let slot = by_provider
                .entry(super::provider(account.harness.as_str()))
                .or_default();
            slot.0 += 1;
            slot.1 += u64::try_from(
                storage
                    .profiles()
                    .list_by_account(&account.id)
                    .map_or(0, |profiles| profiles.len()),
            )
            .unwrap_or(0);
        }
        for (wire, (accounts_n, recipes_n)) in by_provider {
            if let Some(provider) = Provider::from_wire(wire) {
                out.push(StatGauge::AccountsConfigured {
                    provider,
                    count: accounts_n,
                });
                out.push(StatGauge::LaunchSettingsCount {
                    provider,
                    count: recipes_n,
                });
            }
        }
    }

    /// The environment gauges, read when a week closes (spec §7.2):
    /// counts are bucketed by the schema and nothing names an Account,
    /// a project or a machine.
    pub fn gauges(state: &Arc<AppState>) -> Vec<juggler_stats::StatGauge> {
        use juggler_stats::{
            CleanupTierTier, CliVersion, CrashReportsSettingValue, FleetRoleValue,
            PackRedactionPolicyValue, Provider, StatGauge, UpdateAutoCheckValue,
            UpdateChannelValue, WorktreeIsolationEnabledValue,
        };
        let mut out = Vec::new();
        let Ok(storage) = state.storage.lock() else {
            return out;
        };
        let accounts = storage.accounts().list().unwrap_or_default();
        account_gauges(&storage, &accounts, &mut out);
        out.push(StatGauge::ProjectsCount {
            count: u64::try_from(storage.projects().list().map_or(0, |p| p.len())).unwrap_or(0),
        });
        out.push(StatGauge::RenewalDatesEntered {
            count: u64::try_from(accounts.iter().filter(|a| a.renewal_date.is_some()).count())
                .unwrap_or(0),
        });
        let policy = match crate::commands::redaction_global_of(&storage) {
            Some(juggler_handover::redaction::RedactionPolicy::Strip) => "redact",
            Some(juggler_handover::redaction::RedactionPolicy::Keep) => "keep",
            Some(juggler_handover::redaction::RedactionPolicy::AskEachTime) => "ask",
            _ => "other",
        };
        if let Some(value) = PackRedactionPolicyValue::from_wire(policy) {
            out.push(StatGauge::PackRedactionPolicy { value });
        }
        let worktrees = storage
            .app_settings()
            .get("juggler_worktrees")
            .ok()
            .flatten()
            .is_some_and(|raw| raw.trim().len() > 2);
        drop(storage);
        if let Some(value) =
            WorktreeIsolationEnabledValue::from_wire(if worktrees { "on" } else { "off" })
        {
            out.push(StatGauge::WorktreeIsolationEnabled { value });
        }
        let tiers = state.cleanup_settings.all().unwrap_or_default();
        let clean_safe = tiers.iter().filter(|(_, t)| t.tier1_opt_in).count();
        let total = accounts.len();
        for (wire, n) in [
            ("clean_safe", clean_safe),
            ("notify_only", total.saturating_sub(clean_safe)),
        ] {
            if let Some(tier) = CleanupTierTier::from_wire(wire) {
                out.push(StatGauge::CleanupTier {
                    tier,
                    count: u64::try_from(n).unwrap_or(0),
                });
            }
        }
        let peers = crate::fleet::fleet_peers_impl(state).map_or(0, |peers| peers.len());
        out.push(StatGauge::FleetPairedMachines {
            count: u64::try_from(peers).unwrap_or(0),
        });
        let role = if peers == 0 {
            "none"
        } else if crate::link::link_status_impl(state).enabled {
            "node"
        } else {
            "console"
        };
        if let Some(value) = FleetRoleValue::from_wire(role) {
            out.push(StatGauge::FleetRole { value });
        }
        let wire = match crate::updates::channel_of(state) {
            crate::updates::Channel::Stable => "stable",
            // Pre-release channels share the schema's `beta` value.
            crate::updates::Channel::Beta | crate::updates::Channel::Nightly => "beta",
        };
        if let Some(value) = UpdateChannelValue::from_wire(wire) {
            out.push(StatGauge::UpdateChannel { value });
        }
        let auto = if crate::updates::check_on_launch(state) {
            "on"
        } else {
            "off"
        };
        if let Some(value) = UpdateAutoCheckValue::from_wire(auto) {
            out.push(StatGauge::UpdateAutoCheck { value });
        }
        // AJ-E1107: the crash-report setting (only the setting).
        if let Some(value) = CrashReportsSettingValue::from_wire(&crate::crash::effective(state)) {
            out.push(StatGauge::CrashReportsSetting { value });
        }
        // One `--version` per Provider, once a week, on this low-priority
        // thread; only `major.minor` (or `other`) is kept.
        for install in crate::commands::providers_discover_impl(state).unwrap_or_default() {
            if let (Some(provider), Some(version)) = (
                Provider::from_wire(super::provider(&install.provider)),
                install.version,
            ) {
                out.push(StatGauge::ProviderCliVersion {
                    provider,
                    version: CliVersion::parse(&version),
                });
            }
        }
        out
    }

    /// One tick of the weekly cycle (close weeks, upload at most one).
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Io`] on storage failures.
    pub fn tick(
        state: &Arc<AppState>,
        env: &Environment,
        envelope: &Envelope,
        transport: &dyn Transport,
        now_ms: i64,
    ) -> AppResult<juggler_stats::cycle::TickOutcome> {
        let choice = super::choice(state);
        let applied = effective(mode_of(&choice.mode), env);
        if let Some(recorder) = juggler_stats::current() {
            recorder.flush();
        }
        let endpoints = Endpoints::for_build(env);
        let gauges = || gauges(state);
        // AJ-E1109: the personal sync, while sharing and the user wants it.
        let sync_url = crate::figurehead::stats_sync_url(state);
        let token = (applied.mode == Mode::Share
            && crate::figurehead::feature_on(state, "usageSync"))
        .then(|| {
            crate::figurehead::session(state)
                .ok()
                .map(|(_, token)| token)
        })
        .flatten();
        let unlinked = || crate::figurehead::unlinked_by_service(state);
        let ctx = Context {
            effective: applied,
            consent_current: choice.consent_version == super::CONSENT_VERSION,
            consent_version: choice.consent_version,
            envelope,
            endpoints: endpoints.as_ref(),
            transport,
            now_ms,
            gauges: &gauges,
            pow_budget: juggler_stats::pow::BUDGET,
            sync: token.as_ref().map(|token| juggler_stats::cycle::Sync {
                url: &sync_url,
                bearer: token.expose(),
                unlinked: &unlinked,
            }),
        };
        juggler_stats::cycle::tick(&super::stats_dir(state), &ctx).map_err(io)
    }

    /// The app's own cost, sampled each hour: memory now, and CPU used
    /// since the previous sample (percent of one core).
    #[derive(Default)]
    struct OwnCost {
        system: Option<sysinfo::System>,
    }

    impl OwnCost {
        fn sample(&mut self) {
            use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate};
            if !juggler_stats::recording() {
                self.system = None;
                return;
            }
            let pid = Pid::from_u32(std::process::id());
            let first = self.system.is_none();
            let system = self.system.get_or_insert_with(sysinfo::System::new);
            system.refresh_processes_specifics(
                ProcessesToUpdate::Some(&[pid]),
                true,
                ProcessRefreshKind::nothing()
                    .without_tasks()
                    .with_memory()
                    .with_cpu(),
            );
            let Some(process) = system.process(pid) else {
                return;
            };
            #[allow(clippy::cast_precision_loss)] // megabytes into a bucket
            let rss_mb = process.memory() as f64 / (1024.0 * 1024.0);
            super::observe("perf.rss_peak_mb", &[], rss_mb);
            if !first {
                super::observe("perf.cpu_overhead_pct", &[], f64::from(process.cpu_usage()));
            }
        }
    }

    /// The tick at launch, then hourly, on its own lowest-priority
    /// background thread (a puzzle, when the service asks for one, is
    /// solved there). In `off` each tick returns at once and touches
    /// nothing.
    pub fn spawn_ticks(state: Arc<AppState>) {
        let _ = std::thread::Builder::new()
            .name("usage-stats-tick".into())
            .spawn(move || {
                let _ = thread_priority::set_current_thread_priority(
                    thread_priority::ThreadPriority::Min,
                );
                let env = Environment::from_process();
                let transport = StatsTransport::production();
                let envelope = envelope();
                let mut own = OwnCost::default();
                loop {
                    let now = juggler_core::model::Timestamp::now().millis();
                    own.sample();
                    if let Err(e) = tick(&state, &env, &envelope, &transport, now) {
                        eprintln!("ai-juggler: {e}");
                    }
                    std::thread::sleep(std::time::Duration::from_secs(3_600));
                }
            });
    }
}

/// The public list of every metric (opened from Settings ▸ Privacy).
pub const METRICS_DOC_URL: &str =
    "https://github.com/figurehead-ai/AI-juggler/blob/main/docs/usage-stats.md";

/// One counter or gauge of this week, as Settings ▸ Privacy lists it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct UsageStatRow {
    /// `environment` | `features` | `reliability` (spec §6.2).
    pub category: String,
    /// The metric.
    pub metric: String,
    /// Its dimensions, readable (`provider: codex · kind: terminal`).
    pub dims: String,
    /// The count, or the gauge's bucket.
    pub value: String,
}

/// One report file and its state.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct UsageReportRow {
    /// The ISO week.
    pub week: String,
    /// `pending` | `sent` | `skipped`.
    pub status: String,
    /// Why it was skipped, in plain words.
    pub reason: Option<String>,
    /// When a pending report may be uploaded, epoch ms.
    #[ts(type = "number | null")]
    pub upload_at_ms: Option<i64>,
    /// When a failed upload is retried, epoch ms.
    #[ts(type = "number | null")]
    pub next_try_ms: Option<i64>,
    /// Failed attempts so far.
    pub attempts: u32,
    /// Its size, bytes.
    #[ts(type = "number")]
    pub bytes: u64,
    /// It went to the member's Figurehead account, not anonymously.
    pub synced: bool,
}

/// The personal stats sync as Settings ▸ Privacy shows it (AJ-E1109).
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct UsageSyncInfo {
    /// Offered: sharing, this Mac is linked, and the account includes it.
    pub offered: bool,
    /// The user turned it on.
    pub on: bool,
    /// The linked account's email ("Synced to …").
    pub email: Option<String>,
    /// Why it stopped, in plain words.
    pub notice: Option<String>,
}

/// Everything Settings ▸ Privacy shows about usage statistics.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct UsageStatsOverview {
    /// The choice and what applies.
    pub info: UsageStatsInfo,
    /// This ISO week.
    pub week: String,
    /// This week's counters and gauges so far.
    pub rows: Vec<UsageStatRow>,
    /// Every report on this Mac, newest first.
    pub reports: Vec<UsageReportRow>,
    /// When the next pending report may go, epoch ms.
    #[ts(type = "number | null")]
    pub next_upload_ms: Option<i64>,
    /// The last upload's failure, in plain words (cleared by a success).
    pub last_error: Option<String>,
    /// The public metrics document.
    pub docs_url: String,
    /// The personal stats sync.
    pub sync: UsageSyncInfo,
}

/// The report category of a metric (spec §6.2).
#[must_use]
pub fn category(metric: &str, gauge: bool) -> &'static str {
    let reliability = metric.starts_with("perf.")
        || metric.starts_with("stats.")
        || matches!(
            metric,
            "error.class"
                | "app.recovered_after_crash"
                | "storage.migration"
                | "usage.parse_degraded"
                | "fleet.link_degraded"
        );
    if gauge {
        "environment"
    } else if reliability {
        "reliability"
    } else {
        "features"
    }
}

/// A skipped report's reason in plain words.
#[must_use]
pub fn reason_words(reason: &str) -> String {
    match reason {
        "local" => "Counted while stats stayed on this computer — never sent".to_owned(),
        "partial_consent" => "Sharing didn't cover the whole week — never sent".to_owned(),
        "expired" => "Too old to send".to_owned(),
        "not_sampled" => "Not picked this week (sampling) — never sent".to_owned(),
        "server_disabled" => "Figurehead isn't collecting right now".to_owned(),
        "version" => "This version of AI Juggler isn't collected".to_owned(),
        "schema" => "Figurehead no longer accepts this kind of report".to_owned(),
        "mode_changed" => "You stopped sharing before it was sent".to_owned(),
        "gave_up" => "Couldn't send it in time".to_owned(),
        "invalid" => "It didn't pass the checks — never sent".to_owned(),
        "unlinked" => {
            "This computer is no longer linked, so stats sync is off — never sent".to_owned()
        }
        other => other.strip_prefix("rejected:").map_or_else(
            || "Not sent".to_owned(),
            |status| format!("Figurehead refused it ({status})"),
        ),
    }
}

/// Dimensions JSON as readable text.
#[cfg(feature = "stats")]
fn readable_dims(dims: &str) -> String {
    serde_json::from_str::<std::collections::BTreeMap<String, String>>(dims)
        .map(|map| {
            map.iter()
                .map(|(k, v)| format!("{k}: {v}"))
                .collect::<Vec<_>>()
                .join(" · ")
        })
        .unwrap_or_default()
}

/// Core: what Settings ▸ Privacy shows. Reads only; creates nothing.
///
/// # Errors
///
/// Returns [`AppError::Io`] when the store can't be read.
pub fn usage_stats_overview_impl(state: &Arc<AppState>) -> AppResult<UsageStatsOverview> {
    let info = usage_stats_info_impl(state);
    let now = juggler_core::model::Timestamp::now().millis();
    #[cfg(feature = "stats")]
    {
        let dir = stats_dir(state);
        let io = |e: juggler_stats::StoreError| AppError::Io {
            context: "usage stats",
            message: e.to_string(),
        };
        if let Some(recorder) = juggler_stats::current() {
            recorder.flush();
        }
        let gauges: std::collections::BTreeSet<&str> = juggler_stats::METRICS
            .iter()
            .filter(|m| m.kind == "gauge")
            .map(|m| m.name)
            .collect();
        let rows = juggler_stats::cycle::this_week(&dir, now)
            .map_err(io)?
            .into_iter()
            .map(|(metric, dims, value)| UsageStatRow {
                category: category(&metric, gauges.contains(metric.as_str())).to_owned(),
                dims: readable_dims(&dims),
                metric,
                value,
            })
            .collect();
        let files = juggler_stats::cycle::reports(&dir).map_err(io)?;
        let next_upload_ms = files
            .iter()
            .filter(|f| f.status == "pending")
            .filter_map(|f| match (f.upload_at_ms, f.next_try_ms) {
                (Some(a), Some(b)) => Some(a.max(b)),
                (a, b) => a.or(b),
            })
            .min();
        let last_error =
            juggler_stats::cycle::last_attempt(&dir).and_then(|(_, _, _, status)| match status {
                Some(200 | 202) => None,
                None => Some("Couldn't reach Figurehead.".to_owned()),
                Some(503) => Some("Figurehead isn't accepting reports right now.".to_owned()),
                Some(429) => Some("Figurehead is busy.".to_owned()),
                Some(code) => Some(format!("Figurehead answered {code}.")),
            });
        let reports = files
            .into_iter()
            .map(|f| UsageReportRow {
                week: f.week,
                status: f.status.to_owned(),
                reason: f.reason.as_deref().map(reason_words),
                upload_at_ms: f.upload_at_ms,
                next_try_ms: f.next_try_ms,
                attempts: f.attempts,
                bytes: f.bytes,
                synced: f.synced,
            })
            .collect();
        let (linked, email) = crate::figurehead::usage_sync_offer(state);
        let sync = UsageSyncInfo {
            offered: linked && info.effective == "share",
            on: crate::figurehead::feature_on(state, "usageSync"),
            email,
            notice: juggler_stats::cycle::sync_notice(&dir)
                .map_err(io)?
                .map(|_| "This computer is no longer linked, so stats sync is off.".to_owned()),
        };
        Ok(UsageStatsOverview {
            info,
            week: juggler_stats::week::label(now),
            rows,
            reports,
            next_upload_ms,
            last_error,
            docs_url: METRICS_DOC_URL.to_owned(),
            sync,
        })
    }
    #[cfg(not(feature = "stats"))]
    {
        let _ = (state, now);
        Ok(UsageStatsOverview {
            info,
            week: String::new(),
            rows: Vec::new(),
            reports: Vec::new(),
            next_upload_ms: None,
            last_error: None,
            docs_url: METRICS_DOC_URL.to_owned(),
            sync: UsageSyncInfo::default(),
        })
    }
}

/// Core: one report's exact JSON (read-only).
///
/// # Errors
///
/// Returns [`AppError::Validation`] for a bad status or week, and
/// [`AppError::Io`] when the file can't be read.
pub fn usage_stats_report_impl(
    state: &Arc<AppState>,
    status: &str,
    week: &str,
) -> AppResult<String> {
    #[cfg(feature = "stats")]
    {
        juggler_stats::cycle::read_report(&stats_dir(state), status, week).map_err(|e| {
            if e.kind() == std::io::ErrorKind::InvalidInput {
                AppError::Validation {
                    subject: "usage report",
                    message: "unknown report".to_owned(),
                }
            } else {
                AppError::Io {
                    context: "usage stats",
                    message: e.to_string(),
                }
            }
        })
    }
    #[cfg(not(feature = "stats"))]
    {
        let _ = (state, status, week);
        Err(AppError::Validation {
            subject: "usage report",
            message: "usage statistics are not included in this build".to_owned(),
        })
    }
}

/// Core: "Delete local stats" — removes every counter, report, the salt
/// and the upload log, keeping the chosen mode (counting restarts fresh,
/// so this week is no longer fully shared).
///
/// # Errors
///
/// Returns [`AppError::Io`] when the folder can't be deleted.
pub fn usage_stats_delete_local_impl(state: &Arc<AppState>) -> AppResult<UsageStatsSetResult> {
    delete_local(state)?;
    #[cfg(feature = "stats")]
    enabled::apply(state, &juggler_stats::Environment::from_process())?;
    Ok(UsageStatsSetResult {
        info: usage_stats_info_impl(state),
        message: "Local stats deleted.".to_owned(),
    })
}

/// Core: turns the personal stats sync on or off (AJ-E1109). On needs
/// sharing and a linked Mac whose account includes it.
///
/// # Errors
///
/// Returns [`AppError::Validation`] when it can't be offered.
pub fn usage_stats_set_sync_impl(state: &Arc<AppState>, on: bool) -> AppResult<UsageStatsOverview> {
    if on && usage_stats_info_impl(state).effective != "share" {
        return Err(AppError::Validation {
            subject: "stats sync",
            message: "share usage stats first".to_owned(),
        });
    }
    crate::figurehead::set_usage_sync(state, on)?;
    usage_stats_overview_impl(state)
}

/// Core: the current choice and what applies.
#[must_use]
pub fn usage_stats_info_impl(state: &Arc<AppState>) -> UsageStatsInfo {
    let choice = choice(state);
    #[cfg(feature = "stats")]
    {
        enabled::info(&choice, &juggler_stats::Environment::from_process())
    }
    #[cfg(not(feature = "stats"))]
    {
        UsageStatsInfo {
            included: false,
            chosen: choice.mode,
            effective: "off".to_owned(),
            can_upload: false,
            reason: Some("Not included in this build".to_owned()),
            decided_at_ms: choice.decided_at_ms,
            consent_version: choice.consent_version,
            current_consent_version: CONSENT_VERSION,
        }
    }
}

/// Core: records the user's choice and applies it. Choosing `off` stops
/// the recorder and deletes the whole `stats/` folder.
///
/// # Errors
///
/// Returns [`AppError::Validation`] for an unknown mode or a build
/// without usage statistics, and [`AppError::Io`] when the folder can't
/// be deleted or the store can't be opened.
pub fn usage_stats_set_impl(state: &Arc<AppState>, mode: &str) -> AppResult<UsageStatsSetResult> {
    if !matches!(mode, "off" | "local" | "share") {
        return Err(AppError::Validation {
            subject: "usage stats",
            message: format!("unknown mode {mode:?}"),
        });
    }
    if !cfg!(feature = "stats") && mode != "off" {
        return Err(AppError::Validation {
            subject: "usage stats",
            message: "usage statistics are not included in this build".to_owned(),
        });
    }
    save(
        state,
        &UsageStatsChoice {
            mode: mode.to_owned(),
            decided_at_ms: Some(juggler_core::model::Timestamp::now().millis()),
            consent_version: CONSENT_VERSION,
        },
    )?;
    let message = if mode == "off" {
        delete_local(state)?;
        "Usage stats are off. Local stats deleted.".to_owned()
    } else {
        #[cfg(feature = "stats")]
        enabled::apply(state, &juggler_stats::Environment::from_process())?;
        if mode == "local" {
            "Usage stats are counted on this computer only.".to_owned()
        } else {
            "Usage stats are on. One small report a week will be sent.".to_owned()
        }
    };
    Ok(UsageStatsSetResult {
        info: usage_stats_info_impl(state),
        message,
    })
}

/// Stops the recorder and deletes the whole `stats/` folder.
///
/// # Errors
///
/// Returns [`AppError::Io`] when the folder can't be deleted.
pub fn delete_local(state: &Arc<AppState>) -> AppResult<()> {
    let dir = stats_dir(state);
    #[cfg(feature = "stats")]
    let deleted = juggler_stats::delete_all(&dir);
    #[cfg(not(feature = "stats"))]
    let deleted = match std::fs::remove_dir_all(&dir) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other,
    };
    deleted.map_err(|e| AppError::Io {
        context: "usage stats",
        message: e.to_string(),
    })
}

/// At launch: start the recorder if the choice (after the overrides)
/// isn't off. Off creates nothing.
pub fn start(state: Arc<AppState>, cold_start: std::time::Duration) {
    #[cfg(feature = "stats")]
    {
        if let Err(e) = enabled::apply(&state, &juggler_stats::Environment::from_process()) {
            eprintln!("ai-juggler: {e}");
        }
        // AJ-E1103: what happened while this launch opened, counted now
        // that the recorder runs (nothing, in off).
        count("app.launch", &[]);
        if state.launch_facts.migrated {
            count("storage.migration", &[("outcome", "ok")]);
        }
        if state.launch_facts.recovered {
            count("app.recovered_after_crash", &[]);
        }
        #[allow(clippy::cast_precision_loss)] // milliseconds into a bucket
        observe("perf.cold_start_ms", &[], cold_start.as_millis() as f64);
        enabled::spawn_ticks(state);
    }
    #[cfg(not(feature = "stats"))]
    let _ = (state, cold_start);
}

/// On quit: write what was counted.
pub fn shutdown() {
    #[cfg(feature = "stats")]
    juggler_stats::shutdown();
}

/// The metrics the interface may count (everything else is counted by
/// the backend, where it happens).
pub const UI_METRICS: &[&str] = &[
    "ui.screen_opened",
    "ui.inspector_tab",
    "ui.notification",
    "ui.shortcut_used",
    "cleanup_centre.opened",
    "usage.suggestion",
    "resource.alert",
    "onboarding.login",
    "update.flow",
    "perf.board_interactive_ms",
];

/// Where each schema metric is counted (AJ-E1103 Req.1). The coverage
/// test checks this against the schema and the sources.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Site {
    /// Counted in the backend, at the named place.
    Backend(&'static str),
    /// Counted from the interface (through [`UI_METRICS`]).
    Ui(&'static str),
    /// Both: the backend counts most of it, the interface one step.
    Both(&'static str),
    /// A gauge read when a week closes ([`enabled::gauges`]) or raised
    /// as it changes.
    Gauge(&'static str),
    /// Not counted yet, and why.
    NotYet(&'static str),
}

/// Every schema metric and where it is counted.
pub const SITES: &[(&str, Site)] = &[
    ("accounts.configured", Site::Gauge("Accounts per Provider")),
    (
        "provider.cli_version",
        Site::Gauge("`--version` per Provider, once a week"),
    ),
    (
        "launch_settings.count",
        Site::Gauge("launch settings per Provider"),
    ),
    ("projects.count", Site::Gauge("projects")),
    ("fleet.role", Site::Gauge("none / node (Link on) / console")),
    ("fleet.paired_machines", Site::Gauge("paired Macs")),
    (
        "cleanup.tier",
        Site::Gauge("Accounts opted in to safe cleanup, and the rest"),
    ),
    (
        "auto_restart.enabled",
        Site::NotYet("AI Juggler has no auto-restart setting yet"),
    ),
    (
        "pack.redaction_policy",
        Site::Gauge("the global redaction choice"),
    ),
    (
        "renewal_dates.entered",
        Site::Gauge("Accounts with a renewal date"),
    ),
    (
        "worktree.isolation_enabled",
        Site::Gauge("on once Juggler has made a worktree"),
    ),
    ("update.channel", Site::Gauge("the build's channel")),
    ("update.auto_check", Site::Gauge("check on launch")),
    ("ui.density", Site::Gauge("the interface, on change")),
    ("ui.theme", Site::Gauge("the interface, on change")),
    (
        "crash_reports.setting",
        Site::Gauge("`never` until AJ-E1107 adds crash reports"),
    ),
    (
        "instance.peak_concurrent",
        Site::Gauge("the 5 s resource sample, raised as it grows"),
    ),
    (
        "instance.started",
        Site::Backend("every place an Instance row is created (7)"),
    ),
    (
        "instance.ended",
        Site::Backend("commands::apply_lifecycle_event"),
    ),
    (
        "instance.minutes",
        Site::Backend("commands::apply_lifecycle_event"),
    ),
    (
        "instance.state_entered",
        Site::Backend("apply_attention_transition; quota exits"),
    ),
    (
        "instance.auto_restarted",
        Site::NotYet("AI Juggler doesn't auto-restart Instances yet"),
    ),
    (
        "instance.running_model",
        Site::Backend("commands::persist_running_model (family only)"),
    ),
    (
        "instance.wrong_account_blocked",
        Site::Backend("identity::assert_identity"),
    ),
    (
        "session.tokens",
        Site::Backend("usage_pipeline: new Measured rows only"),
    ),
    (
        "switch.started",
        Site::Backend("commands::switch_execute_core"),
    ),
    (
        "switch.completed",
        Site::Backend("switch_execute_core; switch_undo_impl"),
    ),
    (
        "switch.failed_stage",
        Site::Backend("commands::switch_execute_core"),
    ),
    (
        "switch.back",
        Site::Backend("commands::switch_back_execute_impl"),
    ),
    (
        "switch.divergence_shown",
        Site::Backend("commands::switch_back_plan_impl"),
    ),
    (
        "switch.duration_s",
        Site::Backend("commands::switch_execute_core"),
    ),
    ("pack.size_kb", Site::Backend("commands::pack_export_impl")),
    (
        "pack.redaction_reviewed",
        Site::Backend("commands::record_redaction_decision"),
    ),
    (
        "context_sync.applied",
        Site::NotYet("there is no context-sync operation yet"),
    ),
    (
        "usage.wall_hit",
        Site::Backend("commands::step_account_alerts"),
    ),
    (
        "usage.forecast_shown",
        Site::Backend("the quota_display command (Usage tab)"),
    ),
    ("usage.suggestion", Site::Ui("SwitchTargetRows")),
    (
        "usage.threshold_alert",
        Site::Backend("commands::step_account_alerts"),
    ),
    (
        "usage.parse_degraded",
        Site::Backend("usage_pipeline::record_health"),
    ),
    (
        "renewal.warning_shown",
        Site::Backend("commands::step_account_alerts"),
    ),
    (
        "process.children_per_instance",
        Site::Backend("the 5 s resource sample, hourly"),
    ),
    (
        "leftover.detected",
        Site::Backend("the orphan tracker's confirmations"),
    ),
    ("leftover.cleaned", Site::Backend("kill_execute_as")),
    ("leftover.verify_refused", Site::Backend("kill_execute_as")),
    ("cleanup.memory_freed_mb", Site::Backend("kill_execute_as")),
    ("cleanup.ports_freed", Site::Backend("kill_execute_as")),
    (
        "resource.alert",
        Site::Both("cleanup_alerts_inner (shown); Cleanup Centre (acted)"),
    ),
    ("cleanup_centre.opened", Site::Ui("App")),
    (
        "onboarding.scan_completed",
        Site::Backend("commands::environment_scan_full_impl"),
    ),
    (
        "onboarding.found",
        Site::Backend("commands::environment_scan_full_impl"),
    ),
    (
        "onboarding.source_detected",
        Site::Backend("commands::environment_scan_full_impl"),
    ),
    (
        "onboarding.proposal",
        Site::Backend("the funnel event the wizard records"),
    ),
    (
        "onboarding.import",
        Site::Backend("the account_import command"),
    ),
    ("onboarding.login", Site::Ui("FirstRun")),
    (
        "onboarding.expert_opened",
        Site::Backend("the funnel event"),
    ),
    (
        "onboarding.first_launch_s",
        Site::Backend("the funnel's first_launch"),
    ),
    ("onboarding.flow", Site::Backend("the funnel's flow steps")),
    (
        "fleet.pairing",
        Site::Backend("link: host and joiner outcomes"),
    ),
    (
        "fleet.remote_action",
        Site::Backend("remote::send_action; fleet_switch_execute"),
    ),
    (
        "fleet.link_degraded",
        Site::Backend("link: adopt_connection; a dropped link"),
    ),
    ("ui.screen_opened", Site::Ui("App")),
    ("ui.inspector_tab", Site::Ui("RightInspector")),
    (
        "ui.rail_tab",
        Site::NotYet("D-25 removed the rail's tabs; the schema keeps the name"),
    ),
    ("ui.notification", Site::Ui("NotificationRegion")),
    ("ui.shortcut_used", Site::Ui("App; RightInspector")),
    ("app.launch", Site::Backend("usage_stats::start")),
    (
        "app.recovered_after_crash",
        Site::Backend("usage_stats::start (the recovery pass)"),
    ),
    (
        "error.class",
        Site::Backend("AppError crossing to the interface"),
    ),
    (
        "update.flow",
        Site::Both("updates (shown, installed, failed); the notice (deferred)"),
    ),
    (
        "storage.migration",
        Site::Backend("usage_stats::start (opening the database)"),
    ),
    (
        "stats.dropped",
        Site::Backend("juggler-stats: the recorder's flush"),
    ),
    ("stats.truncated", Site::Backend("juggler-stats: assembly")),
    (
        "stats.invalid_report",
        Site::Backend("juggler-stats: assembly"),
    ),
    ("perf.cold_start_ms", Site::Backend("usage_stats::start")),
    (
        "perf.board_interactive_ms",
        Site::Ui("App: the first board load"),
    ),
    (
        "perf.rss_peak_mb",
        Site::Backend("the hourly tick: this app's memory"),
    ),
    (
        "perf.cpu_overhead_pct",
        Site::Backend("the hourly tick: this app's CPU"),
    ),
];

/// Core: one interface event. Only [`UI_METRICS`], only values the schema
/// lists (a `provider` dimension may be any Provider id: it is mapped,
/// unknown ones to `other`); anything else is ignored.
pub fn usage_stats_ui_event_impl(
    metric: &str,
    dims: &std::collections::BTreeMap<String, String>,
    value: Option<f64>,
) {
    let Some(metric) = UI_METRICS.iter().copied().find(|m| *m == metric) else {
        return;
    };
    #[cfg(feature = "stats")]
    if recording() {
        let pairs: Vec<(&str, &str)> = dims
            .iter()
            .map(|(k, v)| {
                let v = if k == "provider" {
                    provider(v)
                } else {
                    v.as_str()
                };
                (k.as_str(), v)
            })
            .collect();
        if let Some(event) = juggler_stats::StatEvent::parse(metric, &pairs, value) {
            juggler_stats::record(event);
        }
    }
    #[cfg(not(feature = "stats"))]
    let _ = (metric, dims, value);
}

/// Core: an interface display preference (`ui.density`, `ui.theme`) as
/// this week's gauge.
pub fn usage_stats_ui_gauge_impl(metric: &str, value: &str) {
    #[cfg(feature = "stats")]
    if recording() {
        use juggler_stats::{StatGauge, UiDensityValue, UiThemeValue};
        let gauge = match metric {
            "ui.density" => {
                UiDensityValue::from_wire(value).map(|value| StatGauge::UiDensity { value })
            }
            "ui.theme" => UiThemeValue::from_wire(value).map(|value| StatGauge::UiTheme { value }),
            _ => None,
        };
        if let Some(gauge) = gauge {
            juggler_stats::set_gauge(gauge);
        }
    }
    #[cfg(not(feature = "stats"))]
    let _ = (metric, value);
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    /// The recorder is process-wide: tests that start or stop it run one
    /// at a time.
    fn serial() -> std::sync::MutexGuard<'static, ()> {
        static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
        SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn state() -> (tempfile::TempDir, Arc<AppState>) {
        let dir = tempfile::tempdir().unwrap();
        let state = Arc::new(AppState::init(dir.path().join("data")).unwrap());
        (dir, state)
    }

    #[test]
    fn default_is_off_and_creates_nothing() {
        let _serial = serial();
        let (_dir, state) = state();
        let info = usage_stats_info_impl(&state);
        assert_eq!(info.chosen, "off");
        assert_eq!(info.effective, "off");
        assert_eq!(info.consent_version, 0);
        start(Arc::clone(&state), std::time::Duration::ZERO);
        assert!(!stats_dir(&state).exists());
    }

    #[cfg(feature = "stats")]
    #[test]
    fn choosing_off_deletes_the_folder_and_says_so() {
        let _serial = serial();
        let (_dir, state) = state();
        let on = usage_stats_set_impl(&state, "local").unwrap();
        assert_eq!(on.info.chosen, "local");
        assert_eq!(on.info.consent_version, CONSENT_VERSION);
        assert!(stats_dir(&state).join("counters.sqlite").exists());
        let off = usage_stats_set_impl(&state, "off").unwrap();
        assert_eq!(off.message, "Usage stats are off. Local stats deleted.");
        assert!(!stats_dir(&state).exists());
        assert!(usage_stats_set_impl(&state, "sometimes").is_err());
    }

    /// A local HTTP server answering `responses` in order; returns each
    /// raw request (head and body).
    #[cfg(feature = "stats")]
    fn serve(responses: Vec<&'static str>) -> (String, std::thread::JoinHandle<Vec<Vec<u8>>>) {
        use std::io::{Read as _, Write as _};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let handle = std::thread::spawn(move || {
            let mut seen = Vec::new();
            for response in responses {
                let (mut stream, _) = listener.accept().unwrap();
                let mut raw = Vec::new();
                let mut buffer = [0_u8; 8192];
                let head_end = loop {
                    let read = stream.read(&mut buffer).unwrap();
                    raw.extend_from_slice(&buffer[..read]);
                    if let Some(at) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
                        break at + 4;
                    }
                    assert!(read > 0, "connection closed mid-request");
                };
                let head = String::from_utf8_lossy(&raw[..head_end]).to_ascii_lowercase();
                let length: usize = head
                    .lines()
                    .find_map(|l| l.strip_prefix("content-length:"))
                    .map_or(0, |v| v.trim().parse().unwrap());
                while raw.len() < head_end + length {
                    let read = stream.read(&mut buffer).unwrap();
                    raw.extend_from_slice(&buffer[..read]);
                }
                stream.write_all(response.as_bytes()).unwrap();
                seen.push(raw);
            }
            seen
        });
        (base, handle)
    }

    #[cfg(feature = "stats")]
    fn http(status: &str, body: &str) -> &'static str {
        Box::leak(
            format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .into_boxed_str(),
        )
    }

    /// AJ-E1104/E1108: a debug build sends to the local endpoint only —
    /// config first, then the gzip report with the user agent and no
    /// cookies — and files it under `sent/`.
    #[cfg(feature = "stats")]
    #[test]
    fn a_debug_build_uploads_to_the_local_endpoint_with_the_contract_headers() {
        use std::io::Read as _;
        let _serial = serial();
        let (_dir, state) = state();
        usage_stats_set_impl(&state, "share").unwrap();
        let config = r#"{"enabled":true,"sample_rate":1.0,"schemas":[1],"min_app_version":"0.0.1","pow_bits":0}"#;
        let (base, server) = serve(vec![
            http("200 OK", config),
            http("202 Accepted", r#"{"ok":true}"#),
        ]);
        let env = juggler_stats::Environment {
            debug_build: true,
            endpoint: Some(format!("{base}/v1/reports")),
            ..juggler_stats::Environment::default()
        };
        // Sharing since before last week, which has a counter.
        let dir = stats_dir(&state);
        let now = juggler_core::model::Timestamp::now().millis();
        let last = juggler_stats::week::label(now - 7 * 86_400_000);
        let store = juggler_stats::Store::open(&dir).unwrap();
        store.set_meta("share_since", "0").unwrap();
        store.add(&last, "stats.truncated", "{}", 4).unwrap();
        let start = juggler_stats::week::start_ms(&last).unwrap();
        let (y, m, d) = juggler_stats::week::civil(start.div_euclid(86_400_000));
        store.mark_active(&format!("{y:04}-{m:02}-{d:02}")).unwrap();
        let transport = enabled::StatsTransport::production();
        // Just after the week closed: assembled, not yet due.
        let closed = juggler_stats::week::end_ms(&last).unwrap() + 1;
        let first = enabled::tick(&state, &env, &enabled::envelope(), &transport, closed).unwrap();
        assert!(first.uploads.is_empty(), "{first:?}");
        assert_eq!(first.assembled.len(), 1, "{first:?}");
        store
            .set_meta(&format!("upload_at:{last}"), &(now - 1).to_string())
            .unwrap();
        let second = enabled::tick(&state, &env, &enabled::envelope(), &transport, now).unwrap();
        assert_eq!(
            second.uploads,
            vec![(last.clone(), juggler_stats::cycle::UploadOutcome::Sent)]
        );
        let requests = server.join().unwrap();
        let get = String::from_utf8_lossy(&requests[0]).to_ascii_lowercase();
        assert!(get.starts_with("get /v1/config.json http/1.1"), "{get}");
        let post = &requests[1];
        let head_end = post.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
        let head = String::from_utf8_lossy(&post[..head_end]).to_ascii_lowercase();
        assert!(head.starts_with("post /v1/reports http/1.1"), "{head}");
        let agent = format!("user-agent: aijuggler/{}", env!("CARGO_PKG_VERSION"));
        assert!(head.contains(&agent), "{head}");
        assert!(get.contains(&agent), "{get}");
        assert!(head.contains("content-type: application/json"), "{head}");
        assert!(head.contains("content-encoding: gzip"), "{head}");
        assert!(!head.contains("cookie"), "{head}");
        let mut json = Vec::new();
        flate2::read::GzDecoder::new(&post[head_end..])
            .read_to_end(&mut json)
            .unwrap();
        assert_eq!(
            json,
            std::fs::read(dir.join("sent").join(format!("{last}.json"))).unwrap()
        );
        let report: serde_json::Value = serde_json::from_slice(&json).unwrap();
        assert_eq!(report["app"]["version"], env!("CARGO_PKG_VERSION"));
        let entries = report["entries"].as_array().unwrap();
        assert!(
            entries.iter().any(|e| e["m"] == "stats.truncated"),
            "{report}"
        );
        // The week's environment gauges ride along (counts as buckets).
        assert!(
            entries
                .iter()
                .any(|e| e["m"] == "projects.count" && e["g"] == "0"),
            "{report}"
        );
        delete_local(&state).unwrap();
    }

    /// AJ-E1104 evidence against a running stats server — a local
    /// development server, never production from a test. Opt-in:
    /// `AIJ_E2E_STATS_URL=http://localhost:8790 cargo test -p juggler-app
    /// --lib usage_stats::tests::live -- --ignored --nocapture`.
    #[cfg(feature = "stats")]
    #[test]
    #[ignore = "needs a local stats server (AIJ_E2E_STATS_URL)"]
    fn live_stats_server_accepts_a_report() {
        use juggler_stats::cycle::Transport as _;
        let base = std::env::var("AIJ_E2E_STATS_URL").expect("AIJ_E2E_STATS_URL");
        assert!(
            base.contains("localhost") || base.contains("127.0.0.1"),
            "local servers only"
        );
        let _serial = serial();
        let (_dir, state) = state();
        usage_stats_set_impl(&state, "share").unwrap();
        let env = juggler_stats::Environment {
            debug_build: true,
            endpoint: Some(format!("{base}/v1/reports")),
            ..juggler_stats::Environment::default()
        };
        let dir = stats_dir(&state);
        let now = juggler_core::model::Timestamp::now().millis();
        let last = juggler_stats::week::label(now - 7 * 86_400_000);
        let store = juggler_stats::Store::open(&dir).unwrap();
        store.set_meta("share_since", "0").unwrap();
        let start = juggler_stats::week::start_ms(&last).unwrap();
        for day in 0..3 {
            let (y, m, d) = juggler_stats::week::civil(start.div_euclid(86_400_000) + day);
            store.mark_active(&format!("{y:04}-{m:02}-{d:02}")).unwrap();
        }
        store.add(&last, "app.launch", "{}", 3).unwrap();
        store
            .add(
                &last,
                "switch.duration_s",
                r#"{"bucket":"15-60","kind":"provider"}"#,
                2,
            )
            .unwrap();
        let transport = enabled::StatsTransport::production();
        // This workspace is 0.0.1, below the service's minimum; present a
        // release-shaped version (the app's own is skipped as `version`).
        let envelope = juggler_stats::report::Envelope {
            app_version: "0.1.0".to_owned(),
            ..enabled::envelope()
        };
        let closed = juggler_stats::week::end_ms(&last).unwrap() + 1;
        let first = enabled::tick(&state, &env, &envelope, &transport, closed).unwrap();
        println!("assembled: {:?}", first.assembled);
        store
            .set_meta(&format!("upload_at:{last}"), &(now - 1).to_string())
            .unwrap();
        let second = enabled::tick(&state, &env, &envelope, &transport, now).unwrap();
        println!("uploads: {:?}", second.uploads);
        assert_eq!(
            second.uploads,
            vec![(last.clone(), juggler_stats::cycle::UploadOutcome::Sent)]
        );
        let sent = std::fs::read(dir.join("sent").join(format!("{last}.json"))).unwrap();
        println!("report: {}", String::from_utf8_lossy(&sent));
        // The same report again is a duplicate, not a second report.
        let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        std::io::Write::write_all(&mut gzip, &sent).unwrap();
        let again = transport
            .post_gzip_json(&format!("{base}/v1/reports"), &gzip.finish().unwrap(), None)
            .unwrap();
        println!(
            "again: {} {}",
            again.status,
            String::from_utf8_lossy(&again.body)
        );
        assert_eq!(again.status, 200);
        delete_local(&state).unwrap();
    }

    /// AJ-E1105: the overview lists this week, the reports with plain
    /// reasons, reads exact JSON, and "Delete local stats" empties it
    /// while keeping the mode.
    #[cfg(feature = "stats")]
    #[test]
    fn the_privacy_overview_shows_what_was_counted_and_kept() {
        let _serial = serial();
        let (_dir, state) = state();
        let off = usage_stats_overview_impl(&state).unwrap();
        assert!(off.rows.is_empty() && off.reports.is_empty());
        assert!(
            !stats_dir(&state).exists(),
            "reading the overview creates nothing"
        );
        usage_stats_set_impl(&state, "local").unwrap();
        count("app.launch", &[]);
        let now = juggler_core::model::Timestamp::now().millis();
        let last = juggler_stats::week::label(now - 7 * 86_400_000);
        juggler_stats::Store::open(&stats_dir(&state))
            .unwrap()
            .add(&last, "app.launch", "{}", 2)
            .unwrap();
        let release = juggler_stats::Environment::default();
        let transport =
            enabled::StatsTransport(Arc::new(juggler_account::RecordingTransport::default()));
        enabled::tick(&state, &release, &enabled::envelope(), &transport, now).unwrap();
        let overview = usage_stats_overview_impl(&state).unwrap();
        let launch = overview
            .rows
            .iter()
            .find(|r| r.metric == "app.launch")
            .unwrap();
        assert_eq!(
            (launch.category.as_str(), launch.value.as_str()),
            ("features", "1")
        );
        assert!(
            overview
                .rows
                .iter()
                .all(|r| r.category != "environment" || r.value.len() <= 6)
        );
        let report = overview.reports.iter().find(|r| r.week == last).unwrap();
        assert_eq!(report.status, "skipped");
        assert_eq!(
            report.reason.as_deref(),
            Some("Counted while stats stayed on this computer — never sent")
        );
        let json = usage_stats_report_impl(&state, "skipped", &last).unwrap();
        assert!(json.starts_with(r#"{"schema":1"#));
        assert!(usage_stats_report_impl(&state, "skipped", "../../etc").is_err());
        assert_eq!(overview.docs_url, METRICS_DOC_URL);
        let deleted = usage_stats_delete_local_impl(&state).unwrap();
        assert_eq!(deleted.message, "Local stats deleted.");
        assert_eq!(deleted.info.chosen, "local", "the mode is kept");
        let after = usage_stats_overview_impl(&state).unwrap();
        assert!(after.reports.is_empty());
        delete_local(&state).unwrap();
    }

    #[test]
    fn skipped_reasons_read_plainly() {
        assert_eq!(reason_words("rejected:413"), "Figurehead refused it (413)");
        assert_eq!(
            reason_words("not_sampled"),
            "Not picked this week (sampling) — never sent"
        );
        assert_eq!(category("perf.cold_start_ms", false), "reliability");
        assert_eq!(category("ui.theme", true), "environment");
        assert_eq!(category("switch.started", false), "features");
    }

    /// AJ-E1108: the tick thread (where a puzzle is solved) can drop to
    /// the lowest priority without privileges.
    #[cfg(feature = "stats")]
    #[test]
    fn the_tick_thread_runs_at_the_lowest_priority() {
        let lowered = std::thread::spawn(|| {
            thread_priority::set_current_thread_priority(thread_priority::ThreadPriority::Min)
        })
        .join()
        .unwrap();
        assert!(lowered.is_ok(), "{lowered:?}");
    }

    #[test]
    fn mappings_keep_only_schema_words() {
        assert_eq!(provider("claude-code"), "claude-code");
        assert_eq!(provider("my-private-cli"), "other");
        for (model, family) in [
            ("claude-opus-4-1-20250805", "opus"),
            ("Claude Sonnet 4.5", "sonnet"),
            ("claude-3-5-haiku", "haiku"),
            ("gpt-5-codex", "gpt-5-codex"),
            ("gpt-5-mini", "gpt-5-mini"),
            ("gpt-5", "gpt-5"),
            ("glm-4.6", "glm-4"),
            ("glm-5", "glm-5"),
            ("acme-secret-model-v9", "other"),
        ] {
            assert_eq!(model_family(model), family, "{model}");
        }
        assert_eq!(
            leftover_category("/usr/local/bin/node", "vite --port 5173"),
            "dev_server"
        );
        assert_eq!(leftover_category("node", "npx @acme/mcp-server"), "mcp");
        assert_eq!(leftover_category("Google Chrome Helper", ""), "browser");
        assert_eq!(leftover_category("python3", "my-script.py"), "other");
    }

    #[test]
    fn error_classes_come_from_the_variant_never_the_text() {
        use crate::error::AppError;
        let secret = "sk-ant-api03-SECRET /Users/someone/project".to_owned();
        assert_eq!(
            error_class(&AppError::Io {
                context: "hand over",
                message: secret.clone()
            }),
            Some(("switch", "io_error"))
        );
        assert_eq!(
            error_class(&AppError::Io {
                context: "remote action",
                message: secret.clone()
            }),
            Some(("fleet", "io_error"))
        );
        assert_eq!(
            error_class(&AppError::Validation {
                subject: "x",
                message: secret
            }),
            None,
            "a refusal isn't an error"
        );
        assert_eq!(
            error_class(&AppError::Core(juggler_core::CoreError::SchemaTooNew {
                found: 9,
                supported: 1
            })),
            Some(("storage", "version_unsupported"))
        );
        assert_eq!(
            error_class(&AppError::StatePoisoned),
            Some(("ui", "invalid_state"))
        );
    }

    /// The interface can count only its own metrics, only with schema
    /// values; everything else is ignored.
    #[cfg(feature = "stats")]
    #[test]
    fn the_interface_cannot_widen_what_is_counted() {
        let dims = |pairs: &[(&str, &str)]| {
            pairs
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect::<std::collections::BTreeMap<_, _>>()
        };
        let _serial = serial();
        let (_dir, state) = state();
        usage_stats_set_impl(&state, "local").unwrap();
        // Not an interface metric: ignored even though it is in the schema.
        assert!(!UI_METRICS.contains(&"session.tokens"));
        // These would panic in a debug build if they reached `record`
        // unparsed; they return quietly.
        usage_stats_ui_event_impl(
            "session.tokens",
            &dims(&[("provider", "codex"), ("direction", "input")]),
            None,
        );
        usage_stats_ui_event_impl(
            "ui.screen_opened",
            &dims(&[("screen", "/Users/someone")]),
            None,
        );
        usage_stats_ui_event_impl(
            "ui.screen_opened",
            &dims(&[("screen", "fleet"), ("extra", "x")]),
            None,
        );
        usage_stats_ui_gauge_impl("ui.theme", "hot-pink");
        // One that is allowed, to show the path records at all.
        usage_stats_ui_event_impl("ui.screen_opened", &dims(&[("screen", "fleet")]), None);
        juggler_stats::current().unwrap().flush();
        let store = juggler_stats::Store::open(&stats_dir(&state)).unwrap();
        let week = juggler_stats::week::label(juggler_core::model::Timestamp::now().millis());
        let counted: Vec<String> = store
            .counters(&week)
            .unwrap()
            .into_iter()
            .map(|(metric, dims, _)| format!("{metric} {dims}"))
            .collect();
        assert!(
            counted.contains(&r#"ui.screen_opened {"screen":"fleet"}"#.to_owned()),
            "{counted:?}"
        );
        assert!(
            !counted
                .iter()
                .any(|c| c.starts_with("session.tokens") || c.contains("/Users")),
            "{counted:?}"
        );
        assert!(
            store.gauges(&week).unwrap().is_empty(),
            "the unknown theme was dropped"
        );
        // A Provider id is mapped, so an unknown one is `other`, not text.
        let parsed = juggler_stats::StatEvent::parse(
            "usage.suggestion",
            &[
                ("provider", provider("my-private-cli")),
                ("action", "shown"),
            ],
            None,
        );
        assert!(parsed.is_some());
        delete_local(&state).unwrap();
    }

    /// The sources under `dir` with these extensions (tests excluded).
    #[cfg(feature = "stats")]
    fn sources(dir: &std::path::Path, extensions: &[&str]) -> Vec<String> {
        let mut out = Vec::new();
        for entry in std::fs::read_dir(dir).unwrap().filter_map(Result::ok) {
            let path = entry.path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            if path.is_dir() {
                if !["generated", "test", "tests", "e2e"].contains(&name.as_str()) {
                    out.extend(sources(&path, extensions));
                }
            } else if extensions.iter().any(|ext| name.ends_with(ext)) && !name.contains(".test.") {
                out.push(std::fs::read_to_string(&path).unwrap());
            }
        }
        out
    }

    /// `metric` as the first argument of one of `calls` somewhere in
    /// `texts` (whitespace and line breaks allowed).
    #[cfg(feature = "stats")]
    fn called(texts: &[String], calls: &[&str], metric: &str) -> bool {
        let literal = format!("\"{metric}\"");
        texts.iter().any(|text| {
            text.match_indices(&literal).any(|(at, _)| {
                let before = text[..at].trim_end();
                calls
                    .iter()
                    .any(|call| before.ends_with(&format!("{call}(")))
            })
        })
    }

    /// AJ-E1103's quality gate: every schema metric has exactly one
    /// entry in [`SITES`], and every claimed site exists in the sources;
    /// a metric marked not-yet has no emitting call anywhere.
    #[cfg(feature = "stats")]
    #[test]
    fn every_metric_is_accounted_for_and_its_site_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let rust = sources(&root.join("src"), &[".rs"]);
        let stats_crate = sources(&root.join("../juggler-stats/src"), &[".rs"]);
        let ui = sources(&root.join("../../ui/src"), &[".ts", ".tsx"]);
        let backend_calls = ["count", "count_n", "observe"];
        let ui_calls = ["statsEvent", "statsGauge"];
        let schema: std::collections::BTreeSet<&str> =
            juggler_stats::METRICS.iter().map(|m| m.name).collect();
        let listed: Vec<&str> = SITES.iter().map(|(name, _)| *name).collect();
        let unique: std::collections::BTreeSet<&str> = listed.iter().copied().collect();
        assert_eq!(unique.len(), listed.len(), "a metric is listed twice");
        assert_eq!(
            unique, schema,
            "SITES must list exactly the schema's metrics"
        );
        let mut table = Vec::new();
        for (metric, site) in SITES {
            let in_backend = called(&rust, &backend_calls, metric);
            let in_ui = called(&ui, &ui_calls, metric);
            let pascal: String = metric
                .split(['.', '_'])
                .map(|part| {
                    let mut chars = part.chars();
                    chars.next().map_or_else(String::new, |c| {
                        c.to_ascii_uppercase().to_string() + chars.as_str()
                    })
                })
                .collect();
            match site {
                Site::Backend(where_) if where_.starts_with("juggler-stats:") => assert!(
                    stats_crate.iter().any(|text| {
                        text.contains(&format!("\"{metric}\""))
                            || text.contains(&format!("StatEvent::{pascal}"))
                    }),
                    "{metric}: not counted in juggler-stats"
                ),
                Site::Backend(_) => assert!(in_backend, "{metric}: no backend call"),
                Site::Ui(_) => {
                    assert!(in_ui, "{metric}: no interface call");
                    assert!(
                        UI_METRICS.contains(metric),
                        "{metric}: not an interface metric"
                    );
                }
                Site::Both(_) => {
                    assert!(in_backend && in_ui, "{metric}: needs both sites");
                    assert!(
                        UI_METRICS.contains(metric),
                        "{metric}: not an interface metric"
                    );
                }
                Site::Gauge(_) => assert!(
                    rust.iter()
                        .any(|text| text.contains(&format!("StatGauge::{pascal}")))
                        || called(&ui, &ui_calls, metric),
                    "{metric}: no gauge is set"
                ),
                Site::NotYet(_) => assert!(
                    !in_backend && !in_ui,
                    "{metric}: counted, but marked not yet"
                ),
            }
            if !matches!(site, Site::Ui(_) | Site::Both(_)) {
                assert!(
                    !UI_METRICS.contains(metric),
                    "{metric}: the interface may not count it"
                );
            }
            table.push(format!("| `{metric}` | {site:?} |"));
        }
        println!("{}", table.join("\n"));
    }

    /// Off and local make zero stats requests, whatever is pending.
    #[cfg(feature = "stats")]
    #[test]
    fn off_and_local_send_nothing() {
        let _serial = serial();
        let (_dir, state) = state();
        let recording = Arc::new(juggler_account::RecordingTransport::default());
        let transport = enabled::StatsTransport(recording.clone());
        let release = juggler_stats::Environment::default();
        let now = juggler_core::model::Timestamp::now().millis();
        enabled::tick(&state, &release, &enabled::envelope(), &transport, now).unwrap();
        assert!(!stats_dir(&state).exists());
        usage_stats_set_impl(&state, "local").unwrap();
        let dir = stats_dir(&state);
        let last = juggler_stats::week::label(now - 7 * 86_400_000);
        juggler_stats::Store::open(&dir)
            .unwrap()
            .add(&last, "stats.truncated", "{}", 1)
            .unwrap();
        let out = enabled::tick(&state, &release, &enabled::envelope(), &transport, now).unwrap();
        assert_eq!(
            out.assembled[0].1,
            juggler_stats::cycle::Placement::Skipped("local".into())
        );
        assert_eq!(recording.count(), 0);
        delete_local(&state).unwrap();
    }

    #[cfg(feature = "stats")]
    #[test]
    fn overrides_show_their_reason() {
        let choice = UsageStatsChoice {
            mode: "share".to_owned(),
            decided_at_ms: Some(1),
            consent_version: 1,
        };
        let env = juggler_stats::Environment {
            do_not_track: Some("1".to_owned()),
            ..juggler_stats::Environment::default()
        };
        let info = enabled::info(&choice, &env);
        assert_eq!(info.effective, "off");
        assert_eq!(
            info.reason.as_deref(),
            Some("Off because DO_NOT_TRACK is set")
        );
    }
}
