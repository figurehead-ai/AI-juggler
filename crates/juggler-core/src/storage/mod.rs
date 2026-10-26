//! SQLite persistence: open/migrate plus typed repositories.
//!
//! Policy (`docs/plan/QUALITY.md` §1, §7):
//! - WAL journal, `NORMAL` sync, foreign keys on.
//! - Forward-only migrations: each entry is one schema version; databases
//!   written by *newer* builds are rejected with
//!   [`CoreError::SchemaTooNew`] instead of being risked.
//! - Corrupt or foreign files degrade to [`CoreError::NotADatabase`].
//! - AJ-E1010: before any migration touches an existing database file, a
//!   consistent copy of it is kept beside it as `<file>.before-v<N>` (N:
//!   the schema being left), the newest [`KEPT_PRE_MIGRATION_COPIES`]
//!   only. The previous version can open that copy, so going back after
//!   an update loses only what changed since. No copy, no migration.

mod account_audit;
mod accounts;
mod agent_access;
mod profiles;
mod sessions;

mod cleanup_audit;
mod cleanup_settings;
mod fleet_sync;
mod footprints;
mod identity;
mod messages;
mod pools;
mod process_ledger;
mod projects;
mod settings;
mod switch_graph;
mod tasks;
mod transcript_index;
mod usage;
mod usage_history;

use std::path::Path;

use rusqlite::Connection;

pub use account_audit::{AccountAuditRecord, AccountAuditRepo};
pub use accounts::AccountsRepo;
pub use agent_access::{AgentAccessRepo, AgentRule, AuditEntry, MADE_BY_USER};
pub use cleanup_audit::{
    ACTOR_AUTO_TIER1, ACTOR_AUTO_TIER2, ACTOR_PREFIX_USER, AuditOutcomeInfo, CleanupAuditRecord,
    CleanupAuditRepo, OUTCOME_PENDING,
};
pub use cleanup_settings::{CleanupSettingsRepo, CleanupTierSettings};
pub use fleet_sync::{FleetSyncRepo, SyncEntry};
pub use footprints::{
    FOOTPRINTS_PER_KEY, Footprint, FootprintStats, FootprintsRepo, MIN_FOOTPRINT_MS,
};
pub use identity::{IdentityAssertion, IdentityRepo};
pub use messages::{
    Doc, HeldAsk, InboxRow, Message, MessageEvent, MessagesRepo, NewMessage, Recipient,
    RecipientState, SENDER_USER, Thread, ThreadState,
};
pub use pools::{
    MemberRef, NO_PROJECT_POOL_ID, Pool, PoolKind, PoolMember, PoolRole, PoolsRepo,
    default_pool_id, default_pool_name, resolve_memberships,
};
pub use process_ledger::{
    EXEC_SETTLE_MS, LedgerRow, ProcessLedgerRepo, ROLE_HELPER, ROLE_LOGIN, ROLE_ROOT, STATE_EXITED,
    STATE_ORPHANED, STATE_RUNNING, STATE_STARTING, STATE_UNVERIFIABLE,
};
pub use profiles::ProfilesRepo;
pub use projects::ProjectsRepo;
pub use projects::{WorktreeHome, project_root, project_root_with, worktree_repo_root};
pub use sessions::{SessionMarks, SessionsRepo};
pub use settings::AppSettingsRepo;
pub use switch_graph::{SwitchGraphRepo, WorkGraph};
pub use tasks::{
    Claim, External, NewTask, TASK_USER, Task, TaskChange, TaskEvent, TaskKey, TaskLink, TaskNeed,
    TaskQuery, TaskSession, TasksRepo,
};
pub use transcript_index::{
    IndexState, IndexedLine, SearchIndexFile, TranscriptIndexRepo, fts_expression, line_text,
};
pub use usage::{CostBucket, Stored, UsageRepo, anchor_settings_key, bucket_key, cap_settings_key};
pub use usage_history::{
    HOUR_MS, HourRow, ModelRow, SETTLED_MS, SwitchRow, TurnRow, UsageHistoryRepo,
    add as add_tokens, thin,
};

use crate::error::CoreError;

/// Schema version 1: accounts, profiles, sessions.
const V1: &str = "
CREATE TABLE accounts (
    id         TEXT PRIMARY KEY,
    harness    TEXT NOT NULL,
    label      TEXT NOT NULL,
    plan       TEXT,
    state_dir  TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE profiles (
    id            TEXT PRIMARY KEY,
    account_id    TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    settings_json TEXT NOT NULL,
    default_cwd   TEXT,
    created_at    INTEGER NOT NULL
);

CREATE TABLE sessions (
    id                  TEXT PRIMARY KEY,
    native_id           TEXT,
    profile_id          TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    machine             TEXT NOT NULL,
    cwd                 TEXT NOT NULL,
    status              TEXT NOT NULL,
    started_at          INTEGER NOT NULL,
    last_activity       INTEGER,
    usage_json          TEXT NOT NULL,
    transcript_ref_json TEXT
);
";

/// Schema version 2: profile names.
const V2: &str = "ALTER TABLE profiles ADD COLUMN name TEXT NOT NULL DEFAULT '';";

/// Schema version 3 (ADR-0007): instance kinds + tracked projects, the
/// juggling anchor.
const V3: &str = "
ALTER TABLE sessions ADD COLUMN kind TEXT NOT NULL DEFAULT 'terminal';
ALTER TABLE sessions ADD COLUMN project TEXT;
CREATE TABLE projects (
    id            TEXT PRIMARY KEY,
    root          TEXT NOT NULL UNIQUE,
    name          TEXT NOT NULL,
    created_at    INTEGER NOT NULL,
    last_activity INTEGER
);
";

/// AJ-E307 Req.1: import provenance columns. Existing rows stay NULL —
/// provenance-unknown is the honest backfill; nothing is guessed.
const V21: &str = "
ALTER TABLE accounts ADD COLUMN import_source_path TEXT;
ALTER TABLE accounts ADD COLUMN import_origin TEXT;
ALTER TABLE accounts ADD COLUMN imported_at INTEGER;
";

/// Schema version 22 (AJ-E708): Fleet sync — the node's durable outbox
/// (sequences never reused: AUTOINCREMENT) and the console's replica +
/// per-origin high-water mark under the origin's sync epoch.
const V22: &str = "
CREATE TABLE fleet_outbox (
    seq       INTEGER PRIMARY KEY AUTOINCREMENT,
    kind      TEXT NOT NULL,
    record_id TEXT NOT NULL,
    payload   BLOB NOT NULL,
    wall_ms   INTEGER NOT NULL
);
CREATE INDEX idx_fleet_outbox_record ON fleet_outbox(kind, record_id);
CREATE TABLE fleet_replica (
    origin      TEXT NOT NULL,
    kind        TEXT NOT NULL,
    record_id   TEXT NOT NULL,
    seq         INTEGER NOT NULL,
    payload     BLOB NOT NULL,
    wall_ms     INTEGER NOT NULL,
    received_ms INTEGER NOT NULL,
    PRIMARY KEY (origin, kind, record_id)
);
CREATE TABLE fleet_replica_hw (
    origin TEXT PRIMARY KEY,
    epoch  TEXT NOT NULL,
    seq    INTEGER NOT NULL
);
";

/// Schema version 23 (AJ-E706): machine-aware lineage — which machine
/// an epoch ran on (`NULL` = this Mac) and where an edge's work went.
const V23: &str = "
ALTER TABLE epochs ADD COLUMN machine_id TEXT;
ALTER TABLE switch_edges ADD COLUMN to_machine TEXT;
";

/// Schema version 24 (AJ-E1001): per-Instance identity assertions —
/// who each Instance ran as, at launch, resume and switch (append-only).
const V24: &str = "
CREATE TABLE identity_assertions (
    session_id TEXT NOT NULL,
    account_id TEXT NOT NULL,
    state      TEXT NOT NULL,
    label      TEXT,
    at_ms      INTEGER NOT NULL,
    moment     TEXT NOT NULL
);
CREATE INDEX idx_identity_assertions_session ON identity_assertions(session_id);
";

/// Schema version 25 (AJ-E1004): the Instance status journal — every
/// transition timestamped, so a summary is read, never reconstructed.
const V25: &str = "
CREATE TABLE instance_transitions (
    session_id  TEXT NOT NULL,
    from_status TEXT,
    to_status   TEXT NOT NULL,
    at_ms       INTEGER NOT NULL
);
CREATE INDEX idx_instance_transitions_session ON instance_transitions(session_id);
CREATE INDEX idx_instance_transitions_at ON instance_transitions(at_ms);
";

/// Schema version 26 (AJ-E1005/AJ-E1007): the Instance lifecycle journal
/// — `reviewed`, `archived`, `unarchived` — each one an appended,
/// timestamped event (history, never rewritten). The latest event per
/// Instance is its lifecycle state; the data itself is never touched.
const V26: &str = "
CREATE TABLE instance_lifecycle (
    session_id TEXT NOT NULL,
    event      TEXT NOT NULL,
    at_ms      INTEGER NOT NULL
);
CREATE INDEX idx_instance_lifecycle_session ON instance_lifecycle(session_id);
";

/// Schema version 27 (AJ-E1007): the local transcript search index. A
/// contentless FTS5 table holds the search terms only, never a second
/// copy of the conversation; each row points back at a line of the
/// original transcript (`transcript_lines`), and `transcript_index_state`
/// records how much of each transcript is indexed (incremental).
const V27: &str = "
CREATE VIRTUAL TABLE transcript_fts USING fts5(
    body,
    content = '',
    contentless_delete = 1,
    tokenize = 'unicode61 remove_diacritics 2'
);
CREATE TABLE transcript_lines (
    rowid      INTEGER PRIMARY KEY,
    session_id TEXT NOT NULL,
    offset     INTEGER NOT NULL,
    len        INTEGER NOT NULL
);
CREATE INDEX idx_transcript_lines_session ON transcript_lines(session_id);
CREATE TABLE transcript_index_state (
    session_id    TEXT PRIMARY KEY,
    path          TEXT NOT NULL,
    indexed_bytes INTEGER NOT NULL
);
";

/// Ordered, forward-only migrations; index + 1 is the schema version.
/// Schema version 4: imported-history enrichment on projects (which
/// accounts worked there before AI Juggler, from imported transcripts).
const V4: &str = "ALTER TABLE projects ADD COLUMN history_json TEXT NOT NULL DEFAULT '{}';";

/// Schema version 5 (ADR-0008 renames; AJ-E106): pure `ALTER … RENAME`
/// — identities change, values do not. Forward-only; a v5 DB opened by a
/// v4 build fails with `SchemaTooNew` (never silent corruption).
const V5: &str = "
ALTER TABLE accounts RENAME COLUMN harness TO provider_id;
ALTER TABLE sessions RENAME TO instances;
ALTER TABLE instances RENAME COLUMN native_id TO conversation_id;
ALTER TABLE instances RENAME COLUMN profile_id TO launch_recipe_id;
ALTER TABLE profiles RENAME TO launch_recipes;
";

/// Schema version 6 (AJ-E405): the structured attention reason rides on
/// the Instance row — additive, nullable, default unchanged.
const V6: &str = "ALTER TABLE instances ADD COLUMN attention_json TEXT;";

/// Schema version 7 (AJ-E410, D-08): the per-Account Tier-1 cleanup
/// opt-in — additive column, default `0` = notify-only (the default tier).
const V7: &str = "ALTER TABLE accounts ADD COLUMN cleanup_opt_in INTEGER NOT NULL DEFAULT 0;";

/// Schema version 8 (AJ-E411, D-12): the optional user-entered renewal
/// date — additive nullable column; `NULL` = `Expiry: unknown`.
const V8: &str = "ALTER TABLE accounts ADD COLUMN renewal_date TEXT;";

/// Schema version 9 (AJ-E412): the transcript-observed running model —
/// additive nullable JSON blob (`{model, source}`); `NULL` = not
/// observed (never the launch overlay).
const V9: &str = "ALTER TABLE instances ADD COLUMN running_model_json TEXT;";

/// Schema version 10 (AJ-E414): the desktop window identity — additive
/// nullable JSON blob (`{pid, bundle_id, title, cgwindow_id[, start_time]}`); `NULL`
/// for terminal Instances — plus the app-level settings kv (hotkey
/// bindings and future user preferences; values are strings, namespaced
/// by feature).
const V10: &str = "
ALTER TABLE instances ADD COLUMN window_identity_json TEXT;
CREATE TABLE app_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
";

/// Schema version 11 (AJ-E415): the per-Account live-preview opt-in —
/// additive column, default `0` = off (Screen Recording is consented
/// only by an explicit enable).
const V11: &str = "ALTER TABLE accounts ADD COLUMN preview_opt_in INTEGER NOT NULL DEFAULT 0;";

/// Schema version 12 (AJ-E501, pass2 §5): the persistent process
/// ledger — one row per Intent (before spawn), patched with PID/PGID/
/// OS start time at spawn, helpers upserted per reconciler tick. It
/// survives root exit, app exit, and reboot; exited rows retain 7 days.
const V12: &str = "
CREATE TABLE process_ledger (
    pid INTEGER,
    pgid INTEGER,
    started_at INTEGER,
    exe_path TEXT NOT NULL,
    cmdline_hash TEXT NOT NULL,
    cmdline_trunc TEXT NOT NULL,
    boot_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    instance_id INTEGER,
    provider TEXT NOT NULL,
    account_id TEXT,
    project_id TEXT,
    machine_id TEXT NOT NULL,
    root_pid INTEGER,
    parent_pid INTEGER,
    depth INTEGER NOT NULL DEFAULT 0,
    role TEXT NOT NULL,
    state TEXT NOT NULL,
    listens_on TEXT,
    cwd TEXT NOT NULL,
    peak_rss INTEGER,
    peak_cpu REAL,
    exit_kind TEXT,
    reaped_by_us INTEGER,
    first_seen INTEGER NOT NULL,
    last_seen INTEGER NOT NULL
);
CREATE INDEX process_ledger_session_state ON process_ledger(session_id, state);
CREATE INDEX process_ledger_pid_start ON process_ledger(pid, started_at, boot_id);
CREATE INDEX process_ledger_state_last_seen ON process_ledger(state, last_seen);
";

/// v13 (AJ-E502): escaped-group flag — a helper whose live PGID left
/// the root's group at reconcile time. Sticky once set: containment
/// is void for that row, forever (pass2 §4 F-9).
const V13: &str = "
ALTER TABLE process_ledger ADD COLUMN escaped_group INTEGER NOT NULL DEFAULT 0;
";

/// v14 (AJ-E509): the append-only cleanup audit — intent BEFORE the
/// first signal, completed with outcomes after the ladder. Every
/// kill (manual or automatic) is accountable for 7 days; cmdlines
/// persist HASHED + truncated only (the privacy boundary — full
/// command lines never outlive the confirm sheet).
const V14: &str = "
CREATE TABLE cleanup_audit (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    at_ms INTEGER NOT NULL,
    actor TEXT NOT NULL,
    pid INTEGER NOT NULL,
    pgid INTEGER,
    exe_path TEXT NOT NULL,
    cmdline_hash TEXT NOT NULL,
    cmdline_trunc TEXT NOT NULL,
    session_id TEXT NOT NULL,
    provider TEXT NOT NULL,
    account_id TEXT,
    project_id TEXT,
    machine_id TEXT NOT NULL,
    identity TEXT NOT NULL,
    scope TEXT NOT NULL,
    outcome TEXT NOT NULL,
    ports_before TEXT,
    ports_after TEXT
);
CREATE INDEX cleanup_audit_at ON cleanup_audit(at_ms);
CREATE INDEX cleanup_audit_account ON cleanup_audit(account_id, at_ms);
";

/// v15 (AJ-E508): per-Account cleanup tier settings — TWO separate
/// opt-ins (D-08 / ADR-0010's E508 amendment), both off by default.
/// The E410 prototype's single `accounts.cleanup_opt_in` seeds Tier 1
/// only (its promise was tidy-after-exit); Tier 2 starts off for
/// everyone, including previous opt-ins.
const V15: &str = "
CREATE TABLE IF NOT EXISTS cleanup_settings (
    account_id TEXT PRIMARY KEY,
    tier1_opt_in INTEGER NOT NULL DEFAULT 0,
    tier2_opt_in INTEGER NOT NULL DEFAULT 0
);
INSERT INTO cleanup_settings (account_id, tier1_opt_in, tier2_opt_in)
    SELECT id, cleanup_opt_in, 0 FROM accounts WHERE cleanup_opt_in = 1;
";

/// v16 (AJ-E510): durable down-period marker — a helper orphaned
/// because its root died WHILE Juggler was closed, written only by
/// the recovery pass. Drives the distinct cleanup label ("left
/// behind while Juggler was closed") versus ordinary grace-based
/// orphans, and survives any number of further restarts.
const V16: &str = "ALTER TABLE process_ledger ADD COLUMN down_period INTEGER NOT NULL DEFAULT 0;";

/// v17 (AJ-E511): the fleet composite key — `session_id+machine_id`
/// is the join key for every cross-machine query (the same session
/// may exist on two machines); the index keeps the machine-scoped
/// inventory inside its ≤5 ms budget.
const V17: &str = "CREATE INDEX IF NOT EXISTS process_ledger_session_machine
    ON process_ledger(session_id, machine_id);";

/// v18 (AJ-E606, deep/09 §Q8): the two-clock engine's append-only
/// stores. `usage_events` dedupes on `UNIQUE(account, msg_uuid)`
/// (NULL uuids are distinct — resumed files never double-count);
/// `quota_samples` is the vendor Measured tail; `reset_queue` is the
/// AJ-E611 surface. Additive only — no existing row's semantics move.
const V18: &str = "
CREATE TABLE usage_events (
    id INTEGER PRIMARY KEY,
    account TEXT NOT NULL,
    session TEXT,
    ts INTEGER NOT NULL,
    model TEXT,
    input INTEGER NOT NULL DEFAULT 0,
    output INTEGER NOT NULL DEFAULT 0,
    cache_r INTEGER NOT NULL DEFAULT 0,
    cache_w INTEGER NOT NULL DEFAULT 0,
    reasoning INTEGER NOT NULL DEFAULT 0,
    credits REAL,
    usd_est REAL,
    source TEXT NOT NULL,
    confidence TEXT NOT NULL,
    msg_uuid TEXT,
    UNIQUE(account, msg_uuid)
);
CREATE TABLE quota_samples (
    account TEXT NOT NULL,
    window TEXT NOT NULL,
    ts INTEGER NOT NULL,
    used_pct REAL NOT NULL,
    resets_at INTEGER,
    confidence TEXT NOT NULL,
    PRIMARY KEY (account, window, ts)
);
CREATE TABLE reset_queue (
    id TEXT PRIMARY KEY,
    session TEXT NOT NULL,
    account TEXT NOT NULL,
    resume_at INTEGER NOT NULL,
    state TEXT NOT NULL DEFAULT 'queued',
    created_at INTEGER NOT NULL
);
CREATE INDEX idx_usage_acct_ts ON usage_events(account, ts);
";

/// v19 (AJ-E609): user pricing overrides — same shape as the bundled
/// ledger rows; a user row supersedes bundled rows for its model from
/// its effective instant (query-time join; history reprices,
/// `usage_events` never change).
const V19: &str = "
CREATE TABLE pricing_overrides (
    model TEXT NOT NULL,
    effective_from_ms INTEGER NOT NULL,
    input_per_1k REAL NOT NULL,
    output_per_1k REAL NOT NULL,
    cache_read_per_1k REAL NOT NULL DEFAULT 0,
    cache_write_per_1k REAL NOT NULL DEFAULT 0,
    source_url TEXT NOT NULL DEFAULT '',
    captured TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (model, effective_from_ms)
);
";

/// Schema version 20 (AJ-E201; pass2/02 §2.3): the Switch Graph —
/// Work items, instance-epochs, append-only switch edges (with
/// two-parent merge lineage), and per-edge artifact manifests.
/// Existing tables are untouched; rows migrate with zero data loss.
const V20: &str = "
CREATE TABLE work_items (
    id         TEXT PRIMARY KEY,
    project    TEXT NOT NULL REFERENCES projects(id),
    title      TEXT NOT NULL,
    created_at INTEGER NOT NULL
);
CREATE INDEX idx_work_items_project ON work_items(project);

CREATE TABLE epochs (
    id                TEXT PRIMARY KEY,
    work_id           TEXT NOT NULL REFERENCES work_items(id),
    session_ref       TEXT,
    native_id         TEXT,
    account           TEXT NOT NULL,
    provider          TEXT NOT NULL,
    resumed_from_epoch TEXT
);
CREATE INDEX idx_epochs_work ON epochs(work_id);

CREATE TABLE switch_edges (
    id               TEXT PRIMARY KEY,
    work_id          TEXT NOT NULL REFERENCES work_items(id),
    from_epoch       TEXT NOT NULL REFERENCES epochs(id),
    to_epoch         TEXT REFERENCES epochs(id),
    from_account     TEXT NOT NULL,
    to_account       TEXT NOT NULL,
    from_provider    TEXT NOT NULL,
    to_provider      TEXT NOT NULL,
    mechanism        TEXT NOT NULL,
    pack_id          TEXT,
    fidelity_shown   TEXT,
    manifest_sha256  TEXT,
    state            TEXT NOT NULL,
    fail_stage       TEXT,
    fail_reason      TEXT,
    actor            TEXT NOT NULL,
    parent_edge_a    TEXT REFERENCES switch_edges(id),
    parent_edge_b    TEXT REFERENCES switch_edges(id),
    created_at       INTEGER NOT NULL
);
CREATE INDEX idx_switch_edges_work ON switch_edges(work_id);

CREATE TABLE artifact_manifests (
    edge        TEXT NOT NULL REFERENCES switch_edges(id),
    rel_path    TEXT NOT NULL,
    sha256      TEXT NOT NULL,
    bytes       INTEGER NOT NULL,
    kind        TEXT NOT NULL,
    disposition TEXT NOT NULL,
    PRIMARY KEY (edge, rel_path)
);
";

/// Schema version 28 (ADR-0018): an account can use the user's own
/// folder where it is. Existing accounts are copies (0).
const V28: &str = "ALTER TABLE accounts ADD COLUMN in_place INTEGER NOT NULL DEFAULT 0;";

/// v29 (AJ-E1406): footprints — how heavy each launch setting's sessions
/// really are on this Mac (one row per session, the newest 50 per key).
const V29: &str = "
CREATE TABLE footprints (
    session_id   TEXT PRIMARY KEY,
    key          TEXT NOT NULL,
    provider     TEXT NOT NULL,
    account_id   TEXT,
    peak_rss     INTEGER NOT NULL,
    p95_cpu      REAL,
    peak_helpers INTEGER NOT NULL,
    duration_ms  INTEGER NOT NULL,
    recorded_at  INTEGER NOT NULL
);
CREATE INDEX idx_footprints_key ON footprints(key, recorded_at);
";

/// v30 (AJ-E1501): managing accounts. A removed account is forgotten,
/// not deleted (its sessions' history stays readable); a project can
/// name the account its new sessions start with; every change to an
/// account is an audit row.
const V30: &str = "
ALTER TABLE accounts ADD COLUMN removed_at INTEGER;
ALTER TABLE projects ADD COLUMN default_account TEXT;
CREATE TABLE account_audit (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id TEXT NOT NULL,
    action     TEXT NOT NULL,
    detail     TEXT,
    actor      TEXT NOT NULL,
    at         INTEGER NOT NULL
);
CREATE INDEX idx_account_audit_account ON account_audit(account_id, at);
";

/// v31 (AJ-E1502): naming and tidying sessions and projects. A session's
/// label shows over its own title; pins lead the rail; a forgotten
/// project is hidden until it is used again. A project's label is its
/// `name` (the folder is never renamed).
const V31: &str = "
ALTER TABLE instances ADD COLUMN label TEXT;
ALTER TABLE instances ADD COLUMN pinned_at INTEGER;
ALTER TABLE projects ADD COLUMN pinned_at INTEGER;
ALTER TABLE projects ADD COLUMN forgotten_at INTEGER;
";

/// v32 (AJ-E1519): the repository each of another manager's worktrees
/// (Claude Squad's, Vibe Kanban's) belongs to, remembered while that
/// manager knows it, so the worktree's sessions stay with their project
/// after it's removed. Folders only.
const V32: &str = "
CREATE TABLE worktree_homes (
    folder TEXT PRIMARY KEY,
    repo TEXT NOT NULL,
    manager TEXT NOT NULL,
    seen_at INTEGER NOT NULL
);
";

/// v33 (AJ-E1702): pools. The pools module owns its tables
/// ([`pools::MIGRATION`]); they take their place in the one sequence here.
const V33: &str = pools::MIGRATION;

/// v34 (E18): messages between agents, their recipients and threads, the
/// append-only event log, shared docs, pool links, held asks and pool
/// pauses. The messages module owns its tables ([`messages::MIGRATION`]).
const V34: &str = messages::MIGRATION;

/// v35: kept for D-41's sightings (`e24-processes`, built at the same
/// time): that branch's migration takes this place when it merges, so
/// the numbers stay one sequence. Until then it is a no-op (a database
/// made on this branch alone is a test's, never a release's).
const V35: &str = "";

/// v36 (E19): the tasks a pool shares, their links, the sessions that
/// worked them, the append-only event log, and what waits on the user.
/// The tasks module owns its tables ([`tasks::MIGRATION`]).
const V36: &str = tasks::MIGRATION;

const MIGRATIONS: &[&str] = &[
    V1, V2, V3, V4, V5, V6, V7, V8, V9, V10, V11, V12, V13, V14, V15, V16, V17, V18, V19, V20, V21,
    V22, V23, V24, V25, V26, V27, V28, V29, V30, V31, V32, V33, V34, V35, V36,
];

/// How many pre-migration copies (`<file>.before-v<N>`) are kept beside a
/// database: the newest by schema.
pub const KEPT_PRE_MIGRATION_COPIES: usize = 2;

/// The pre-migration copies kept beside the database at `db`, newest
/// schema first: `(schema, path)`.
#[must_use]
pub fn pre_migration_copies(db: &Path) -> Vec<(i64, std::path::PathBuf)> {
    let (Some(dir), Some(name)) = (db.parent(), db.file_name()) else {
        return Vec::new();
    };
    let prefix = format!("{}.before-v", name.to_string_lossy());
    let mut copies: Vec<(i64, std::path::PathBuf)> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let file = entry.file_name().to_string_lossy().into_owned();
            let schema = file.strip_prefix(&prefix)?.parse::<i64>().ok()?;
            Some((schema, entry.path()))
        })
        .collect();
    copies.sort_by_key(|copy| std::cmp::Reverse(copy.0));
    copies
}

/// Keeps a consistent copy (SQLite's online backup API, so what is still
/// only in the WAL is in it too) of the database about to be migrated, as
/// `<file>.before-v<current>`: written to a temporary name, flushed to disk
/// with its folder, owner-only, then renamed — a half-written copy never
/// carries the name. Then prunes to the newest
/// [`KEPT_PRE_MIGRATION_COPIES`]. On failure the temporary file is removed
/// and nothing is migrated.
fn keep_pre_migration_copy(conn: &Connection, db: &Path, current: i64) -> Result<(), CoreError> {
    let name = db
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let target = db.with_file_name(format!("{name}.before-v{current}"));
    let partial = db.with_file_name(format!("{name}.before-v{current}.partial"));
    let _ = std::fs::remove_file(&partial);
    let written = (|| -> Result<(), CopyFailure> {
        {
            let mut copy = Connection::open(&partial)?;
            let backup = rusqlite::backup::Backup::new(conn, &mut copy)?;
            backup.run_to_completion(256, std::time::Duration::ZERO, None)?;
        }
        owner_only(&partial);
        // A writable handle: Windows flushes (FlushFileBuffers) only
        // through one, and a read-only flush fails there.
        std::fs::OpenOptions::new()
            .write(true)
            .open(&partial)?
            .sync_all()?;
        std::fs::rename(&partial, &target)?;
        // The rename survives a power cut once the folder is flushed. Only
        // Unix opens a folder as a file; Windows has no such flush.
        #[cfg(unix)]
        if let Some(dir) = db.parent()
            && let Ok(folder) = std::fs::File::open(dir)
        {
            let _ = folder.sync_all();
        }
        Ok(())
    })();
    if let Err(failure) = written {
        let _ = std::fs::remove_file(&partial);
        let hint = if failure.disk_full {
            "the disk is full — free some space, then open AI Juggler again"
        } else {
            "nothing was changed"
        };
        return Err(CoreError::Io(format!(
            "couldn't keep a copy of the data before upgrading it ({}); {hint}{}",
            failure.message,
            if failure.disk_full {
                "; nothing was changed"
            } else {
                ""
            }
        )));
    }
    for (_, stale) in pre_migration_copies(db)
        .into_iter()
        .skip(KEPT_PRE_MIGRATION_COPIES)
    {
        let _ = std::fs::remove_file(stale);
    }
    Ok(())
}

/// Why a pre-migration copy couldn't be written.
struct CopyFailure {
    message: String,
    disk_full: bool,
}

impl From<rusqlite::Error> for CopyFailure {
    fn from(error: rusqlite::Error) -> Self {
        let disk_full = matches!(
            &error,
            rusqlite::Error::SqliteFailure(failure, _) if failure.code == rusqlite::ErrorCode::DiskFull
        );
        Self {
            message: error.to_string(),
            disk_full,
        }
    }
}

impl From<std::io::Error> for CopyFailure {
    fn from(error: std::io::Error) -> Self {
        Self {
            disk_full: error.kind() == std::io::ErrorKind::StorageFull,
            message: error.to_string(),
        }
    }
}

/// Owner-only (`0600`) on Unix: AI Juggler's data and its copies are no one
/// else's to read. Best effort; elsewhere a no-op.
fn owner_only(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    #[cfg(not(unix))]
    let _ = path;
}

/// How long a connection waits for another connection's lock before a
/// statement fails with "database is locked" (AJ-E1521). The app opens
/// `juggler.sqlite` four times (storage, process ledger, cleanup audit,
/// cleanup settings), so a write on one regularly meets a write on
/// another. Set explicitly, not left to the driver's default.
pub const BUSY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Applies [`BUSY_TIMEOUT`] to a connection on the app database and
/// switches it to WAL.
///
/// # Errors
///
/// [`CoreError::Storage`] when SQLite refuses either setting.
pub(crate) fn wait_for_locks(conn: &Connection) -> Result<(), CoreError> {
    conn.busy_timeout(BUSY_TIMEOUT)
        .and_then(|()| enable_wal(conn))
        .map_err(CoreError::Storage)
}

/// Switches `conn` to WAL, waiting up to [`BUSY_TIMEOUT`] for another
/// connection's lock. SQLite answers this one statement "busy" at once —
/// it skips the busy handler while upgrading a lock it might deadlock
/// on — so the wait is a retry here.
fn enable_wal(conn: &Connection) -> rusqlite::Result<()> {
    let deadline = std::time::Instant::now() + BUSY_TIMEOUT;
    loop {
        match conn.pragma_update(None, "journal_mode", "WAL") {
            Err(rusqlite::Error::SqliteFailure(error, _))
                if error.code == rusqlite::ErrorCode::DatabaseBusy
                    && std::time::Instant::now() < deadline =>
            {
                // poll: bounded by BUSY_TIMEOUT; only while another
                // connection holds the lock the switch needs.
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            other => return other,
        }
    }
}

/// An opened AI Juggler database with typed repositories.
#[derive(Debug)]
pub struct Storage {
    conn: Connection,
    migrated: bool,
}

impl Storage {
    /// Opens (creating if needed) and migrates the database at `path`.
    ///
    /// # Errors
    ///
    /// - [`CoreError::NotADatabase`] if the file exists but is not a valid
    ///   database.
    /// - [`CoreError::SchemaTooNew`] if the database needs a newer build.
    /// - [`CoreError::Storage`] for other SQLite failures.
    pub fn open(path: &Path) -> Result<Self, CoreError> {
        let conn = Connection::open(path).map_err(|e| CoreError::NotADatabase {
            path: path.to_path_buf(),
            source: e,
        })?;
        owner_only(path);
        Self::initialize(conn, Some(path))
    }

    /// Opens an in-memory database (tests and ephemeral indexes).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn open_in_memory() -> Result<Self, CoreError> {
        let conn = Connection::open_in_memory()?;
        Self::initialize(conn, None)
    }

    /// Applies connection pragmas and runs migrations.
    fn initialize(conn: Connection, path: Option<&Path>) -> Result<Self, CoreError> {
        // Everything in this phase touches the file for the first time, so
        // any failure here means "not a database" for on-disk paths.
        // The wait comes first: switching to WAL itself needs the lock.
        let current: i64 = conn
            .busy_timeout(BUSY_TIMEOUT)
            .and_then(|()| enable_wal(&conn))
            .and_then(|()| conn.pragma_update(None, "synchronous", "NORMAL"))
            .and_then(|()| conn.pragma_update(None, "foreign_keys", "ON"))
            .and_then(|()| conn.query_row("PRAGMA user_version", [], |row| row.get(0)))
            .map_err(|e| match path {
                Some(p) => CoreError::NotADatabase {
                    path: p.to_path_buf(),
                    source: e,
                },
                None => CoreError::Storage(e),
            })?;
        let supported = i64::try_from(MIGRATIONS.len()).unwrap_or(i64::MAX);
        if let Some(path) = path
            && current > 0
            && current < supported
        {
            keep_pre_migration_copy(&conn, path, current)?;
        }
        let mut storage = Self {
            conn,
            migrated: false,
        };
        storage.migrated = storage.migrate(current)?;
        storage.conn.execute_batch(usage::DERIVED)?;
        storage.conn.execute_batch(usage_history::HISTORY_DERIVED)?;
        Ok(storage)
    }

    /// Opening this database ran at least one migration (a new database
    /// or an upgrade) — AJ-E1103 counts it.
    #[must_use]
    pub const fn migrated_on_open(&self) -> bool {
        self.migrated
    }

    /// Runs pending forward migrations from the current schema version;
    /// `true` when any ran.
    fn migrate(&self, current: i64) -> Result<bool, CoreError> {
        let supported = i64::try_from(MIGRATIONS.len()).unwrap_or(i64::MAX);
        if current > supported {
            return Err(CoreError::SchemaTooNew {
                found: current,
                supported,
            });
        }
        // A negative user_version means a tampered database; treating it as 0
        // re-runs migrations and fails cleanly on the existing tables.
        let applied = usize::try_from(current).unwrap_or(0);
        for (idx, sql) in MIGRATIONS.iter().enumerate().skip(applied) {
            let version = i64::try_from(idx).unwrap_or(0) + 1;
            let tx = self.conn.unchecked_transaction()?;
            tx.execute_batch(sql)?;
            tx.pragma_update(None, "user_version", version)?;
            tx.commit()?;
        }
        Ok(applied < MIGRATIONS.len())
    }

    /// Repository for [`crate::model::Account`] rows.
    #[must_use]
    pub fn accounts(&self) -> AccountsRepo<'_> {
        AccountsRepo::new(&self.conn)
    }

    /// Repository for the account audit rows (AJ-E1501).
    #[must_use]
    pub fn account_audit(&self) -> AccountAuditRepo<'_> {
        AccountAuditRepo::new(&self.conn)
    }

    /// Repository for [`crate::model::Profile`] rows.
    #[must_use]
    pub fn profiles(&self) -> ProfilesRepo<'_> {
        ProfilesRepo::new(&self.conn)
    }

    /// Repository for [`crate::model::SessionRecord`] rows.
    #[must_use]
    pub fn sessions(&self) -> SessionsRepo<'_> {
        SessionsRepo::new(&self.conn)
    }

    /// Repository for [`crate::model::ProjectRecord`] rows.
    #[must_use]
    pub fn projects(&self) -> ProjectsRepo<'_> {
        ProjectsRepo::new(&self.conn)
    }

    /// Repository for app-level settings (schema v10 kv).
    #[must_use]
    pub fn app_settings(&self) -> AppSettingsRepo<'_> {
        AppSettingsRepo::new(&self.conn)
    }

    /// Repository for usage events & quota samples (schema v18,
    /// AJ-E606): append-only with uuid dedupe.
    #[must_use]
    pub fn usage(&self) -> UsageRepo<'_> {
        UsageRepo::new(&self.conn)
    }

    /// The usage history's summary queries (0.1.5): read-only sums per
    /// hour or day, never the raw events.
    #[must_use]
    pub fn usage_history(&self) -> UsageHistoryRepo<'_> {
        UsageHistoryRepo::new(&self.conn)
    }

    /// Repository for the Switch Graph (schema v20, AJ-E201):
    /// Work items, epochs, append-only switch edges, manifests.
    #[must_use]
    pub fn switch_graph(&self) -> SwitchGraphRepo<'_> {
        SwitchGraphRepo::new(&self.conn)
    }

    /// Repository for the local transcript search index (schema v27,
    /// AJ-E1007).
    #[must_use]
    pub fn transcript_index(&self) -> TranscriptIndexRepo<'_> {
        TranscriptIndexRepo::new(&self.conn)
    }

    /// Repository for per-Instance identity assertions (schema v24,
    /// AJ-E1001).
    #[must_use]
    pub fn identity(&self) -> IdentityRepo<'_> {
        IdentityRepo::new(&self.conn)
    }

    /// Repository for pools, their members and grants (schema v33,
    /// AJ-E1702).
    #[must_use]
    pub fn pools(&self) -> PoolsRepo<'_> {
        PoolsRepo::new(&self.conn)
    }

    /// Repository for what agents may do in each pool: levels, rules,
    /// approvals and the audit (schema v33, D-28).
    #[must_use]
    pub fn agent_access(&self) -> AgentAccessRepo<'_> {
        AgentAccessRepo::new(&self.conn)
    }

    /// Repository for messages between agents, their docs, links, held
    /// asks and pauses (schema v34, E18).
    #[must_use]
    pub fn messages(&self) -> MessagesRepo<'_> {
        MessagesRepo::new(&self.conn)
    }

    /// Repository for the tasks a pool shares (schema v36, E19).
    #[must_use]
    pub fn tasks(&self) -> TasksRepo<'_> {
        TasksRepo::new(&self.conn)
    }

    /// Repository for footprints (schema v29, AJ-E1406).
    #[must_use]
    pub fn footprints(&self) -> FootprintsRepo<'_> {
        FootprintsRepo::new(&self.conn)
    }

    /// Repository for Fleet sync (schema v22, AJ-E708): the outbox and
    /// the replica of paired machines' records.
    #[must_use]
    pub fn fleet_sync(&self) -> FleetSyncRepo<'_> {
        FleetSyncRepo::new(&self.conn)
    }

    /// Raw connection access for migrations/fixture tooling (tests,
    /// diagnostics). Normal writes go through the repositories.
    #[must_use]
    pub fn raw_conn(&self) -> &Connection {
        &self.conn
    }

    /// The schema version this build writes (and the newest it opens).
    #[must_use]
    pub fn schema_version() -> i64 {
        i64::try_from(MIGRATIONS.len()).unwrap_or(i64::MAX)
    }

    /// The schema version of this open database.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn user_version(&self) -> Result<i64, CoreError> {
        Ok(self
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))?)
    }

    /// AJ-E1009: writes a consistent snapshot of the whole database to
    /// `path` (`VACUUM INTO`; the file must not exist).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn snapshot_to(&self, path: &Path) -> Result<(), CoreError> {
        self.conn
            .execute("VACUUM INTO ?1", [path.to_string_lossy().as_ref()])?;
        Ok(())
    }

    /// AJ-E1009: replaces this database's contents with the database at
    /// `path` (SQLite's online backup API, page by page). The source is
    /// opened — and migrated forward — first, so an older backup lands at
    /// this build's schema.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::SchemaTooNew`] for a newer source, or
    /// [`CoreError::Storage`] on SQLite failures.
    pub fn restore_from(&mut self, path: &Path) -> Result<(), CoreError> {
        drop(Self::open(path)?); // migrates the staged copy in place
        self.conn.restore(
            rusqlite::MAIN_DB,
            path,
            None::<fn(rusqlite::backup::Progress)>,
        )?;
        self.conn.pragma_update(None, "foreign_keys", "ON")?;
        Ok(())
    }

    /// AJ-E1010: going back to an earlier version — replaces this
    /// database's contents with the older database at `path` **as it is**
    /// (no migration; its schema comes with it), through SQLite's online
    /// backup API. Only for handing the data to an older build: the caller
    /// must stop using this storage at once (the app quits).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn replace_with_older(&mut self, path: &Path) -> Result<(), CoreError> {
        self.conn.restore(
            rusqlite::MAIN_DB,
            path,
            None::<fn(rusqlite::backup::Progress)>,
        )?;
        Ok(())
    }

    /// The full forward-only migration list (diagnostics + upgrade
    /// tests build earlier schema states from it).
    #[must_use]
    pub fn migrations() -> &'static [&'static str] {
        MIGRATIONS
    }
}
