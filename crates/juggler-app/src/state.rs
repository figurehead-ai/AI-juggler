//! Application state: storage, adapter registry, supervisor — the single
//! backend instance behind every Tauri command (spec §2).

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use juggler_adapter::AdapterRegistry;
use juggler_core::model::SessionId;
use juggler_core::storage::Storage;
use juggler_supervisor::tree::LedgerStore as _;
use juggler_supervisor::{
    HeuristicEngine, HeuristicThresholds, LiveStatsCollector, OrphanTracker, Supervisor,
};

/// Everything the commands operate on. `storage` is `Mutex`-wrapped:
/// rusqlite connections are `Send` but not `Sync`.
pub struct AppState {
    /// What happened while this launch opened (AJ-E1103 counts it once
    /// stats start).
    pub launch_facts: LaunchFacts,
    /// App data directory (databases, managed accounts, packs).
    pub data_dir: PathBuf,
    /// SQLite persistence (accounts, profiles, sessions).
    pub storage: Mutex<Storage>,
    /// Registered harness adapters.
    pub registry: AdapterRegistry,
    /// Process supervisor owning launched sessions.
    pub supervisor: Supervisor,
    /// Batched live process stats for the Instance read model (AJ-E401):
    /// owns the persistent process table so CPU% deltas survive ticks.
    /// Lock ordering: never held together with `storage`.
    pub resources: Mutex<LiveStatsCollector>,
    /// The machine's own signals (AJ-E1401): CPU, memory and pressure,
    /// swap, disks in use, power, thermal — sampled on the same 2 s tick —
    /// and the AI tools AI Juggler did not start (AJ-E1402).
    /// Lock ordering: never held together with `resources` or `storage`;
    /// held only for the cheap part of a tick (the disk work, the storage
    /// read and the running-tools scan run outside it).
    pub machine: Mutex<crate::machine_health::MachineWatch>,
    /// Exit snapshots → confirmed orphans + stop states (AJ-E410).
    /// Interior-mutex: safe to read without the collector lock.
    pub orphans: OrphanTracker,
    /// The persistent spawn ledger handle (AJ-E501): queries + prune.
    pub ledger: std::sync::Arc<juggler_supervisor::tree::CoreLedgerStore>,
    /// The append-only cleanup audit (AJ-E509): intent before signal.
    pub audit: std::sync::Arc<juggler_core::storage::CleanupAuditRepo>,
    /// Per-Account cleanup tier opt-ins (AJ-E508, D-08): TWO separate
    /// toggles on their own WAL connection.
    pub cleanup_settings: std::sync::Arc<juggler_core::storage::CleanupSettingsRepo>,
    /// Live Tier-2 notices (AJ-E508 Req.1): one per crash-orphan, the
    /// Undo window's state. Entries exist only while a window is open.
    pub tier2_pending: Mutex<std::collections::HashMap<u32, crate::commands::Tier2Notice>>,
    /// The one live preview session (AJ-E415 Req.5: one at a time).
    /// Lock ordering: never held together with `storage`.
    pub preview: Mutex<Option<crate::commands::PreviewSession>>,
    /// Per-Instance transcript parse offsets for running-model
    /// detection (AJ-E412): how many bytes each tail-scan has consumed,
    /// so the 2 s tick only ever parses NEW lines.
    pub model_offsets: Mutex<std::collections::HashMap<SessionId, u64>>,
    /// Stuck/waiting/quota heuristics (AJ-E405); thresholds user-tunable
    /// via `heuristics.json`.
    pub heuristics: HeuristicEngine,
    /// AJ-E1004 Req.1: per-Provider no-output thresholds (seconds) by
    /// harness id, from `heuristics.json`'s `providers` map; a Provider
    /// without one uses the global threshold.
    pub provider_stuck_secs: std::collections::BTreeMap<String, u64>,
    /// AJ-E1010: where updating stands (in memory; the launch-check
    /// setting lives in app settings).
    pub updates: crate::updates::UpdateManager,
    /// AJ-E1205: held while a Cloud backup uploads (one at a time).
    pub cloud_uploading: Mutex<()>,
    /// AJ-E1007: held for the length of a transcript-indexing pass.
    pub indexing: Mutex<()>,
    /// AJ-E1206: the account's Mac directory and when it was read (read
    /// at most once a minute).
    pub account_directory: Mutex<Option<(i64, juggler_account::DeviceDirectory)>>,
    /// AJ-E1207: every file the last backup export opened (the guardrail
    /// audit: never an Account folder or Provider credential).
    pub export_reads: Mutex<Vec<std::path::PathBuf>>,
    /// Terminal ids with a live output forwarder (dedup: one forwarder per
    /// terminal no matter how often the UI re-attaches).
    pub terminal_taps: Mutex<std::collections::HashSet<String>>,
    /// Live kill-ladder cancel flags by attempt id (AJ-E507 Req.3:
    /// the grace window is cancellable). Entries exist only while a
    /// ladder runs; flipped by `kill_cancel`.
    pub kill_cancels:
        Mutex<std::collections::HashMap<String, std::sync::Arc<std::sync::atomic::AtomicBool>>>,
    /// Fleet identity + signed peer registry (AJ-E702), loaded on first
    /// use. Lock ordering: never held together with `storage`.
    pub fleet: crate::fleet::FleetCell,
    /// The Fleet link (AJ-E704): off until the user turns it on.
    pub link: crate::link::LinkManager,
    /// The optional Figurehead account (AJ-E1202): silent until linked.
    pub figurehead: crate::figurehead::FigureheadManager,
    /// AJ-E808: installed Provider versions from the last discovery, so
    /// a fidelity-register PASS recorded on other versions reads
    /// UNVALIDATED (the upgrade resets it by itself).
    pub provider_versions: Mutex<std::collections::HashMap<String, String>>,
    /// AJ-E1529: every account's conversations, read from the tools' own
    /// files (kept in memory; rebuilt from the files after a restart).
    pub conversations: crate::features::conversations::ConversationIndex,
}

impl AppState {
    /// Builds the state rooted at `data_dir` with the built-in adapters.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::AppError`] when storage or the supervision
    /// journal cannot be opened.
    pub fn init(data_dir: PathBuf) -> Result<Self, crate::error::AppError> {
        let registry = default_registry_with_overlays(data_dir.join(OVERLAYS_DIR));
        Self::init_with_registry(data_dir, registry)
    }

    /// Builds the state with an explicit adapter registry (tests inject
    /// stub-backed adapters here).
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::AppError`] when storage or the supervision
    /// journal cannot be opened.
    pub fn init_with_registry(
        data_dir: PathBuf,
        registry: AdapterRegistry,
    ) -> Result<Self, crate::error::AppError> {
        std::fs::create_dir_all(&data_dir)
            .map_err(|error| crate::error::AppError::io("create app data dir", &error))?;
        let storage = Storage::open(&data_dir.join("juggler.sqlite"))?;
        // AJ-E501: the persistent spawn ledger — its OWN connection to
        // the same WAL store (spawn threads + reconciler ticks write
        // without the app's storage mutex), attached to the supervisor
        // (intent-before-spawn, helper upserts, exit patches).
        let ledger = std::sync::Arc::new(juggler_supervisor::tree::CoreLedgerStore::new(
            juggler_core::storage::ProcessLedgerRepo::open(&data_dir.join("juggler.sqlite"))?,
        ));
        // Retention (Req.5): exited rows older than 7 days at startup;
        // running rows are never pruned. The recurring job rides the
        // E504 reconciler tick.
        let cutoff = juggler_core::model::Timestamp::now().millis() - 7 * 24 * 3_600_000;
        let _ = ledger.prune(cutoff);
        // AJ-E509: the append-only cleanup audit — its OWN WAL
        // connection (the intent write must not queue behind the
        // storage mutex: ≤5 ms p95 in front of the ladder); 7-day
        // retention mirrors the ledger's.
        let audit = std::sync::Arc::new(juggler_core::storage::CleanupAuditRepo::open(
            &data_dir.join("juggler.sqlite"),
        )?);
        let _ = audit.prune(cutoff);
        // AJ-E508: per-Account tier opt-ins — own connection, seeded
        // from the E410 prototype flag by the v15 migration.
        let cleanup_settings = std::sync::Arc::new(
            juggler_core::storage::CleanupSettingsRepo::open(&data_dir.join("juggler.sqlite"))?,
        );
        let supervisor =
            Supervisor::with_journal(&data_dir.join("supervision"))?.with_ledger(ledger.clone());
        // Orphan grace (AJ-E504 Req.3): the user-tunable default, from
        // settings; per-Provider overrides land with the Cleanup
        // Centre's settings surface (AJ-E506). 30 s when unset — the
        // false-alert budget, never lowered for demos.
        if let Some(secs) = storage
            .app_settings()
            .get("orphan_grace_secs")
            .ok()
            .flatten()
            .and_then(|value| value.parse::<u64>().ok())
        {
            supervisor.set_orphan_grace("", secs);
        }
        // AJ-E510: the startup recovery pass (pass2 §11.2) — BEFORE
        // reconciliation, so verified survivors get handles (stoppable
        // through the E507 path, F-3 closed) and the marker set drives
        // everything downstream.
        let report = run_startup_recovery(&ledger, &supervisor, &storage, &audit);
        let launch_facts = LaunchFacts {
            migrated: storage.migrated_on_open(),
            recovered: !(report.adopted.is_empty()
                && report.left_behind.is_empty()
                && report.unverifiable.is_empty()),
        };
        // Crash/restart reconciliation: sessions recorded running whose
        // processes did not survive the restart are marked stopped —
        // never a permanently fake "live" dot (survivors stay running).
        let adopted = supervisor.alive_adopted_sessions();
        {
            let records = storage.sessions().list(None).unwrap_or_default();
            for record in records {
                if matches!(
                    record.status,
                    juggler_core::model::SessionStatus::Starting
                        | juggler_core::model::SessionStatus::Running
                ) && !adopted.contains(&record.id)
                {
                    let mut reconciled = record;
                    reconciled.status = juggler_core::model::SessionStatus::Stopped;
                    reconciled.last_activity = Some(juggler_core::model::Timestamp::now());
                    let _ = storage.sessions().update(&reconciled);
                }
            }
        }
        // AJ-E508 Req.5 (per AJ-E510): any recovery finding suspends
        // Tiers 1-2 for 24 h until the user re-confirms in the Cleanup
        // Centre — identity data is least trustworthy right after a
        // crash, so the suspension is mandatory, not a nicety.
        if !report.adopted.is_empty()
            || !report.left_behind.is_empty()
            || !report.unverifiable.is_empty()
        {
            let suspended_until = now_epoch_ms() + 24 * 3_600_000;
            let _ = storage
                .app_settings()
                .set("cleanup_suspended_until", &suspended_until.to_string());
        }
        // AJ-E1505: Settings › Sessions' "Stuck after", over the file.
        let heuristics = HeuristicEngine::new(crate::settings_policy::with_stored_stuck_after(
            load_thresholds(&data_dir),
            &storage,
        ));
        Ok(Self {
            audit,
            cleanup_settings,
            tier2_pending: Mutex::new(std::collections::HashMap::new()),
            supervisor,
            ledger,
            registry,
            storage: Mutex::new(storage),
            resources: Mutex::new(LiveStatsCollector::new()),
            machine: Mutex::new(crate::machine_health::MachineWatch::default()),
            orphans: OrphanTracker::new(),
            model_offsets: Mutex::new(std::collections::HashMap::new()),
            heuristics,
            provider_stuck_secs: load_provider_thresholds(&data_dir),
            updates: crate::updates::UpdateManager::default(),
            cloud_uploading: Mutex::new(()),
            indexing: Mutex::new(()),
            account_directory: Mutex::new(None),
            export_reads: Mutex::new(Vec::new()),
            terminal_taps: Mutex::new(std::collections::HashSet::new()),
            kill_cancels: Mutex::new(std::collections::HashMap::new()),
            fleet: Mutex::new(None),
            link: crate::link::LinkManager::default(),
            figurehead: crate::figurehead::FigureheadManager::default(),
            provider_versions: Mutex::new(std::collections::HashMap::new()),
            conversations: crate::features::conversations::ConversationIndex::default(),
            preview: Mutex::new(None),
            launch_facts,
            data_dir,
        })
    }

    /// Directory holding managed harness account state.
    #[must_use]
    pub fn accounts_dir(&self) -> PathBuf {
        self.data_dir.join("accounts")
    }

    /// ADR-0018: AI Juggler's own files for in-place accounts (launch
    /// settings, status line capture) — never the user's folder.
    #[must_use]
    pub fn overlays_dir(&self) -> PathBuf {
        self.data_dir.join(OVERLAYS_DIR)
    }

    /// Registers a kill-ladder attempt's cancel flag (AJ-E507): the
    /// UI generates the attempt id, `kill_execute` polls the flag
    /// through the grace window, `kill_cancel` flips it.
    pub fn register_kill_cancel(
        &self,
        attempt: &str,
    ) -> std::sync::Arc<std::sync::atomic::AtomicBool> {
        let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        if let Ok(mut cancels) = self.kill_cancels.lock() {
            cancels.insert(attempt.to_string(), std::sync::Arc::clone(&flag));
        }
        flag
    }

    /// Flips a registered attempt's cancel flag (idempotent; unknown
    /// attempts are no-ops — the ladder already finished).
    pub fn flip_kill_cancel(&self, attempt: &str) {
        if let Ok(cancels) = self.kill_cancels.lock()
            && let Some(flag) = cancels.get(attempt)
        {
            flag.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }

    /// Drops a finished attempt's flag (the registry stays the size
    /// of the live ladders, never a leak).
    pub fn unregister_kill_cancel(&self, attempt: &str) {
        if let Ok(mut cancels) = self.kill_cancels.lock() {
            cancels.remove(attempt);
        }
    }
}

/// What happened while this launch opened.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LaunchFacts {
    /// Opening the database ran a migration.
    pub migrated: bool,
    /// The startup recovery pass found Instances or helpers that outlived
    /// the previous run (it didn't shut down cleanly).
    pub recovered: bool,
}

/// Where in-place accounts' overlays live, under the data dir.
pub const OVERLAYS_DIR: &str = "overlays";

/// The built-in adapter set (community manifests arrive post-v1).
#[must_use]
pub fn default_registry() -> AdapterRegistry {
    registry_with(juggler_adapters::ClaudeCodeAdapter::new())
}

/// The built-in adapters, with in-place accounts' launch overlays under
/// `overlays_root` (ADR-0018).
#[must_use]
pub fn default_registry_with_overlays(overlays_root: PathBuf) -> AdapterRegistry {
    registry_with(juggler_adapters::ClaudeCodeAdapter::new().with_overlays_root(overlays_root))
}

fn registry_with(claude: juggler_adapters::ClaudeCodeAdapter) -> AdapterRegistry {
    let mut registry = AdapterRegistry::new();
    let claude = std::sync::Arc::new(claude);
    registry.register(claude.clone());
    // AJ-E204: the same adapter, coerced to the optional handover
    // sub-trait (transplant mechanics live behind it).
    registry.register_handover(claude);
    // AJ-E1301: Codex, the second Provider; AJ-E1304: its transplant
    // and prime support (every Codex pair stays unvalidated, beta).
    let codex = std::sync::Arc::new(juggler_adapters::CodexAdapter::new());
    registry.register(codex.clone());
    registry.register_handover(codex);
    registry
}

/// The AJ-E510 startup recovery pass (pass2 §11.2): identity-verify the
/// durable ledger's live rows, re-adopt verified survivors (handles with
/// a working verified stop, F-3 closed), mark helpers of roots that died
/// while Juggler was down `Orphaned` with their original attribution,
/// and — when anything was found — persist the ONE-time summary and
/// audit it as `actor=system:recovery`. Identity failures become
/// `Unverifiable` (listed, never killed).
fn run_startup_recovery(
    ledger: &std::sync::Arc<juggler_supervisor::tree::CoreLedgerStore>,
    supervisor: &juggler_supervisor::Supervisor,
    storage: &Storage,
    audit: &std::sync::Arc<juggler_core::storage::CleanupAuditRepo>,
) -> juggler_supervisor::supervisor::RecoveryReport {
    let machine = juggler_app_machine_id(storage);
    // Candidate SESSIONS come from the live-row scan, but the pass
    // reads each session's FULL rows — a root that died while Juggler
    // was down is already `exited` (detached watcher threads patch the
    // durable ledger), and its still-running helpers need that root
    // context to be classified as left-behind.
    let rows_by_session = {
        use juggler_supervisor::tree::LedgerStore as _;
        let mut sessions: Vec<juggler_core::model::SessionId> = Vec::new();
        if let Ok(candidates) = ledger.candidates(&machine) {
            for row in candidates {
                let session = juggler_core::model::SessionId::new(row.session_id.clone());
                if !sessions.contains(&session) {
                    sessions.push(session);
                }
            }
        }
        sessions
            .into_iter()
            .filter_map(|session| {
                ledger
                    .session_rows(&session)
                    .ok()
                    .map(|rows| (session, rows))
            })
            .collect::<Vec<_>>()
    };
    let report = supervisor.recover(&rows_by_session);
    if !report.adopted.is_empty()
        || !report.left_behind.is_empty()
        || !report.unverifiable.is_empty()
    {
        // Req.3: ONE summary, exact template, shown once, audited as
        // actor=system:recovery.
        let summary = format!(
            "{} sessions re-adopted, {} helpers left behind, {} processes could not be verified and were left alone.",
            report.adopted.len(),
            report.left_behind.len(),
            report.unverifiable.len()
        );
        let _ = storage.app_settings().set("recovery_summary", &summary);
        let _ = storage.app_settings().set("recovery_summary_consumed", "0");
        let _ = audit.intent(&juggler_core::storage::CleanupAuditRecord {
            id: 0,
            at_ms: juggler_core::model::Timestamp::now().millis(),
            actor: "system:recovery".into(),
            pid: 0,
            pgid: None,
            exe_path: "(recovery)".into(),
            cmdline_hash: "(recovery)".into(),
            cmdline_trunc: summary.clone(),
            session_id: "(restart)".into(),
            provider: "system".into(),
            account_id: None,
            project_id: None,
            machine_id: machine.as_str().to_string(),
            identity: "startup recovery pass (AJ-E510 §11.2)".into(),
            scope: "summary".into(),
            outcome: summary,
            ports_before: None,
            ports_after: None,
        });
    }
    report
}

/// The local `MachineId` (generated + persisted on first read).
fn juggler_app_machine_id(storage: &Storage) -> juggler_core::model::MachineId {
    use juggler_core::model::MachineId;
    if let Ok(Some(existing)) = storage.app_settings().get("local_machine_id")
        && !existing.is_empty()
    {
        return MachineId::new(existing);
    }
    let generated = format!(
        "machine-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or_default()
    );
    let _ = storage.app_settings().set("local_machine_id", &generated);
    MachineId::new(generated)
}

/// Current epoch milliseconds (the suspension stamp).
fn now_epoch_ms() -> i64 {
    juggler_core::model::Timestamp::now().millis()
}

/// Resolves the per-platform app data directory (spec §11 layout): one
/// per edition (D-23), so AI Juggler Nightly and AI Juggler Dev never
/// open AI Juggler's data.
///
/// # Errors
///
/// Returns a message string when no OS data directory can be determined.
pub fn resolve_data_dir() -> Result<PathBuf, String> {
    dirs::data_dir()
        .map(|base| base.join(crate::edition::current().data_dir_name()))
        .ok_or_else(|| "no OS data directory available".to_string())
}

/// Loads the user-tunable heuristic thresholds from `heuristics.json`;
/// missing/corrupt files fall back to the SPEC §5.1 defaults (never a
/// crash — the Settings screen lands with its own task and writes here).
fn load_thresholds(data_dir: &Path) -> HeuristicThresholds {
    std::fs::read_to_string(data_dir.join("heuristics.json"))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

/// AJ-E1004 Req.1: `heuristics.json`'s optional per-Provider overrides,
/// `{"providers": {"codex": {"stuck_no_output_secs": 900}}}`. A missing
/// or malformed map is empty: every Provider uses the global threshold.
fn load_provider_thresholds(data_dir: &Path) -> std::collections::BTreeMap<String, u64> {
    #[derive(serde::Deserialize)]
    struct Override {
        stuck_no_output_secs: u64,
    }
    #[derive(serde::Deserialize)]
    struct File {
        #[serde(default)]
        providers: std::collections::BTreeMap<String, Override>,
    }
    std::fs::read_to_string(data_dir.join("heuristics.json"))
        .ok()
        .and_then(|raw| serde_json::from_str::<File>(&raw).ok())
        .map(|file| {
            file.providers
                .into_iter()
                .map(|(harness, row)| (harness, row.stuck_no_output_secs))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::{load_provider_thresholds, load_thresholds};

    #[test]
    fn provider_thresholds_ride_alongside_the_global_ones() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load_provider_thresholds(dir.path()).is_empty());
        std::fs::write(
            dir.path().join("heuristics.json"),
            r#"{"stuck_no_output_secs":300,"stuck_cpu_max_pct":5.0,"stuck_transcript_stale_secs":300,"waiting_prompt_secs":120,"providers":{"codex":{"stuck_no_output_secs":900}}}"#,
        )
        .unwrap();
        assert_eq!(load_provider_thresholds(dir.path())["codex"], 900);
        assert_eq!(load_thresholds(dir.path()).stuck_no_output_secs, 300);
    }
}
