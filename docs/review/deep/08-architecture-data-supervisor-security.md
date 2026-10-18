# 08 — Architecture, Data, Supervisor & Security (Deep Review)

> Deep architecture review of AI Juggler at `main` (2026-10-12 baseline).
> Extends `docs/review/full-competitive-ux-review.md` findings F3–F5
> with crate-level evidence. Every factual claim cites `file:line`
> or a specification reference. No code was modified for this review.
> Method: full read of `crates/juggler-core`, `juggler-adapter`,
> `juggler-adapters`, `juggler-supervisor`, `juggler-app` sources.

## Verdict in one paragraph

The layering is right and the discipline is real, but the system is
a single-harness, single-connection, single-process design wearing
multi-harness, multi-session types. The three load-bearing gaps are:
(1) the adapter trait at `crates/juggler-adapter/src/adapter.rs:26-124`
cannot express SPEC §4's `usage`/`export`/`prime` contract, so M2
adapters will fork the trait or bypass it; (2) one `Mutex<Storage>`
at `crates/juggler-app/src/state.rs:17` serializes all SQLite access
while N+1 list queries and full-file transcript rescans grow with
session count; (3) the supervisor's crash/orphan story depends on a
single append-only JSONL journal with PID-only identity, which is
not safe against PID reuse. None of these require a rewrite. All
three require decisions before adapter #2, not after it.

## 1. Current architecture map

Crate graph (from `Cargo.toml:1-11` workspace members and each
crate's `src/lib.rs`):

```text
                    ┌─────────────────────────────────┐
                    │  ui/ React + TS (webview)        │
                    │  invoke() / Tauri events         │
                    └──────────────┬──────────────────┘
                                   │ commands + events
                    ┌──────────────▼──────────────────┐
                    │  juggler-app (Tauri 2 shell)     │
                    │  commands.rs:1313  lib.rs:119    │
                    │  state.rs:108  terminal.rs:213   │
                    │  AppState { Mutex<Storage>,      │
                    │    AdapterRegistry, Supervisor,  │
                    │    terminal_taps }               │
                    └──┬────────┬────────┬───────┬─────┘
                       │        │        │       │
              ┌────────▼──┐ ┌───▼─────┐ ┌▼─────┐ ┌▼──────────┐
              │ juggler-  │ │juggler- │ │jugg- │ │ juggler-  │
              │ supervisor│ │adapters │ │ler-  │ │ handover  │
              │ PTY/piped │ │ Claude  │ │core  │ │ STUB only │
              │ journal   │ │ + fake  │ │model │ │ lib.rs:13 │
              │ ring buf  │ │         │ │SQLite│ │           │
              └─────┬─────┘ └──┬──────┘ └──┬───┘ └───────────┘
                    │          │          │
              ┌─────▼──────────▼──────────▼─────┐
              │  juggler-adapter (trait contract) │
              │  HarnessAdapter, Capabilities,    │
              │  LaunchSpec, SessionWatcher       │
              │          │ imports model from     │
              │          └────► juggler-core      │
              └──────────────────────────────────┘
                    ┌─────────────────────────────────┐
                    │  juggler-link STUB only lib.rs:12│
                    │  (fleet §11, no implementation)  │
                    └─────────────────────────────────┘
```

### 1.1 What actually flows at runtime

- **Launch:** `launch_profile` at
  `crates/juggler-app/src/commands.rs:657-714` looks up profile +
  account, optionally writes a settings overlay to the managed
  state dir (`commands.rs:666-673`), calls
  `adapter.launch_spec` (`commands.rs:686`), spawns via the
  supervisor (`commands.rs:688`), then opens the `Mutex<Storage>`
  lock to touch the project and insert the session row
  (`commands.rs:692-709`). Note the ordering: the process is
  spawned *before* the session row exists, so a crash between
  `spawn` and `insert` leaves a journaled process with no row.
  The startup reconciler at `crates/juggler-app/src/state.rs:58-72`
  cannot see it because it iterates stored rows only.
- **Lifecycle:** one forwarder thread at
  `crates/juggler-app/src/lib.rs:55-70` polls
  `supervisor.recv_timeout(500ms)` and calls
  `apply_lifecycle_event` at `commands.rs:782-810`, which maps
  `Started → Running` and `Exited → Finished/Stopped/Failed`
  only. `WaitingUser`, `Stuck`, `QuotaBlocked` (model at
  `crates/juggler-core/src/model.rs:194-198`) have no producer.
- **Terminal stream:** supervisor ring (`crates/juggler-supervisor/src/ring.rs:11-17`)
  fans out to `terminal.rs` forwarders (`crates/juggler-app/src/terminal.rs:100-145`)
  that batch at 16 ms (`terminal.rs:29`) and base64-encode into
  `terminal://output` Tauri events.
- **Discovery/registry:** `default_registry` at
  `crates/juggler-app/src/state.rs:91-97` registers exactly one
  adapter (`ClaudeCodeAdapter`). `AdapterRegistry` itself
  (`crates/juggler-adapter/src/registry.rs`) is a `Vec` with
  last-wins replacement; there is no manifest loader hook in
  code despite SPEC §4 promising post-v1 TOML manifests.
- **Watching:** `ClaudeTranscriptWatcher`
  (`crates/juggler-adapters/src/claude_code.rs:410-483`) uses
  `notify` for wake-up and an unconditional full-tree rescan as
  the source of truth (`claude_code.rs:438-450`). See §5.

### 1.2 Layering strengths worth protecting

- The trait crate genuinely is the only place harness specifics
  belong: `crates/juggler-adapter/src/adapter.rs:14-22` states the
  read-only-except-`prepare_account` rule and `juggler-core`
  explicitly knows nothing about harnesses
  (`crates/juggler-core/src/lib.rs:1-12`). Keep this boundary.
- `juggler-core::fs::atomic_write` (`crates/juggler-core/src/fs.rs:18-30`)
  is temp-file + `sync_all` + rename, the correct crash-safe
  primitive; `write_settings_overlay`
  (`crates/juggler-adapters/src/claude_code.rs:598-616`) uses it.
- Error types are typed and semver-conscious:
  `CoreError` at `crates/juggler-core/src/error.rs:10-44`
  (`#[non_exhaustive]`, `SchemaTooNew`/`NotADatabase` degrade
  rather than crash), matching QUALITY.md §1 as cited in
  `crates/juggler-core/src/storage/mod.rs:3-8`.

## 2. Data model: ER diagram and field-by-field critique

### 2.1 Entity-relationship diagram (as built, v4)

```text
accounts ──< profiles ──< sessions >── projects (nullable FK, no constraint)
  │             │             │
  │             │             ├─ kind, machine (bare TEXT, no FK)
  │             │             ├─ usage_json (TEXT blob)
  │             │             └─ transcript_ref_json (TEXT blob)
  └─────────────┴── state_dir on disk (managed dirs, 0700)

projects: root UNIQUE, history_json TEXT ('{}' default)
NO TABLES FOR: usage_events, lineage, footprint_stats,
  quota windows, context packs — all promised by SPEC §6/§12
  (docs/SPEC.md:399) but absent from storage/mod.rs:82.
```

### 2.2 Schema definition (verbatim provenance)

Schema lives as string constants in
`crates/juggler-core/src/storage/mod.rs:28-82`: `V1` (accounts,
profiles, sessions), `V2` (`profiles.name`), `V3` (sessions
`kind`/`project`, `projects` table), `V4` (`projects.history_json`).
`MIGRATIONS: &[&str] = &[V1, V2, V3, V4]` at
`crates/juggler-core/src/storage/mod.rs:82`. Pragmas set in
`initialize` at `storage/mod.rs:118-133`: `journal_mode=WAL`,
`synchronous=NORMAL`, `foreign_keys=ON`.

### 2.3 Every table, every field

**`accounts`** (`storage/mod.rs:29-36`, repo `storage/accounts.rs:27-110`):

| Field | Type | Critique |
|---|---|---|
| `id` | TEXT PK | App-generated string; no format guard. Fine, but inserts accept any id — collision handling is a raw SQLite error surfaced as `Storage` (`accounts.rs:27-41`). |
| `harness` | TEXT NOT NULL | No CHECK or FK to a harness registry. `login_state_of` at `commands.rs:867-872` silently returns `LoggedOut` for any non-Claude harness, so a bad `harness` value degrades invisibly. |
| `label` | TEXT NOT NULL | No uniqueness. Two accounts both labelled "Work" are indistinguishable in the rail; `session_infos` uses label as display key (`commands.rs:947`). Add partial unique index on `(harness, label)` or document duplicates as allowed. |
| `plan` | TEXT NULL | `PlanTier` is deliberately opaque (`model.rs:115-140`) — correct for vendor churn, but there is no `plan_verified_at`, so F1.4 expiry/tier drift has nowhere to live. |
| `state_dir` | TEXT NOT NULL | Stored via `to_string_lossy` (`accounts.rs:36,58`). On non-UTF-8 paths this silently mangles the isolation root. Acceptable on macOS; must be documented, and `Account.state_dir: PathBuf` (`model.rs:154-156`) round-trips lossily through storage. |
| `created_at` | INTEGER NOT NULL | Written as `Timestamp::now().millis()` at insert (`accounts.rs:37`) but absent from the `Account` struct (`model.rs:145-157`) — the model cannot answer "when was this account added?" A v5 column already exists in spirit; expose it or drop it. |

**`profiles`** (`storage/mod.rs:38-44`, repo `storage/profiles.rs:32-116`):

| Field | Type | Critique |
|---|---|---|
| `id` | TEXT PK | Same note as accounts. |
| `account_id` | TEXT FK CASCADE | Correct: deleting an account cascades to profiles (`profiles` v1 at `storage/mod.rs:40`) and transitively to sessions. But cascade also destroys session history on account delete — there is no archive path (prior F1.3). Prefer soft-delete for accounts with sessions. |
| `name` | TEXT (v2) | Added by `storage/mod.rs:61` with `DEFAULT ''`. Legacy rows get empty names; `profile_infos` at `commands.rs:895` will render blank. Backfill to label-derived default in v5. |
| `settings_json` | TEXT NOT NULL | `SettingsOverlay{model, env}` (`model.rs:159-168`) serialized wholesale (`profiles.rs:40`). No schema version inside the blob; adding a field is backward-compatible via `#[serde(default)]`, removing one is not. Env values are stored in plaintext SQLite — see §6. |
| `default_cwd` | TEXT NULL | Lossy path conversion again (`profiles.rs:41,134-137`). Also duplicated by `Profile.default_cwd` (`model.rs:182-184`) and launch-time `cwd` argument — `launch_profile` precedence at `commands.rs:678-685` (explicit > profile default > temp dir) is reasonable but undocumented in the type. |
| `created_at` | INTEGER NOT NULL | Same orphan as accounts: stored (`profiles.rs:42`) but not in `Profile` (`model.rs:172-184`). |

**`sessions`** (`storage/mod.rs:46-57` + `66-67`, repo `storage/sessions.rs:38-145`):

| Field | Type | Critique |
|---|---|---|
| `id` | TEXT PK | Generated as `sess-<suffix>` at `commands.rs:687`. Login terminals use `login-<suffix>` (`terminal.rs:67`) but are never inserted as rows — an intentional split that must stay explicit, because `alive_adopted_sessions` only reconciles journaled spawns. |
| `native_id` | TEXT NULL | Dual source of truth with `transcript_ref.native_id` (`model.rs:311-321`). Handover writes `native_id: Some(..)` on the row but `TranscriptRef{native_id: None}` (`commands.rs:1173,1183-1186`) — the two disagree by construction today. Collapse to one: keep `native_id` on the row, drop it from `TranscriptRef`, or make `TranscriptRef` authoritative and drop the column. |
| `profile_id` | TEXT FK CASCADE | Deleting a profile deletes its sessions. Combined with account cascade, one profile delete can erase a project's entire history. Needs an archive/retention state before delete is exposed broadly. |
| `machine` | TEXT NOT NULL | `MachineId` (`model.rs:63-66`) hard-coded `"this-mac"` at `commands.rs:699,1175`. Harmless now; fleet §11 will need a real `machines` table, and a bare TEXT with no FK will accept garbage until then. Add CHECK or accept the debt explicitly. |
| `kind` | TEXT (v3) | `InstanceKind::{Terminal,Desktop}` (`model.rs:323-355`), parse failures are hard errors in `map_session` (`sessions.rs:159-167`), while `SessionStatus` parse failures are also hard errors (`sessions.rs:172-180`). This contradicts the "degrade, never panic" intent at `model.rs:242-243` — one unknown enum value makes `list()` fail for *every* session. Map unknown → sentinel + warning instead. |
| `project` | TEXT NULL | **No foreign key.** v3 adds the column bare (`storage/mod.rs:67`) with no `REFERENCES projects(id)`. Orphaned project ids are possible today; `projects_list_impl` joins in memory (`commands.rs:1212+`), so orphans silently render as "no project". v5 must add the FK (table rebuild required — see §3). |
| `cwd` | TEXT NOT NULL | Lossy (`sessions.rs:54`). Also the handover encoder does `replace('/','-')` on this string (`commands.rs:1121`) — two lossy steps composed. |
| `status` | TEXT NOT NULL | Nine-value enum (`model.rs:186-207`). Correct size; missing `Archived` for lifecycle (F1.3) and no `status_changed_at`, so "waiting how long?" (the Needs-Attention sort key in F2.5) cannot be answered. |
| `started_at` / `last_activity` | INTEGER | Epoch millis via `Timestamp` (`model.rs:74-113`). `last_activity` is set only on lifecycle transitions (`commands.rs:807`); transcript-only activity (watcher events) does not update it, so `last_activity` under-reports truly active sessions. |
| `usage_json` | TEXT NOT NULL | `UsageSnapshot{tokens, window, confidence}` (`model.rs:299-309`). Sound for a snapshot, but SPEC §6 requires an append-only `usage_events` table (`docs/SPEC.md` Storage layout) — snapshots alone cannot produce burn-down or per-day rollups. This is the single largest data-model gap vs SPEC. |
| `transcript_ref_json` | TEXT NULL | Path + native id as JSON. Path is harness-owned and read-only by contract (`model.rs:311-321`) — correct. But no `transcript_mtime`/`bytes` cache, so freshness checks must stat the file every time. |

**`projects`** (`storage/mod.rs:68-74` + `80`, repo `storage/projects.rs`):

| Field | Type | Critique |
|---|---|---|
| `id` | TEXT PK | `proj-<suffix>` generated in `touch` (`projects.rs` touch impl). Fine. |
| `root` | TEXT NOT NULL UNIQUE | Canonical root is the juggling anchor (ADR-0007, `model.rs:367-384`). UNIQUE is correct. But `touch` does read-then-insert/update without a transaction (`projects.rs` touch impl) — two concurrent launches in a new directory can race to the UNIQUE constraint. Wrap in `INSERT … ON CONFLICT(root) DO UPDATE`. |
| `name` | TEXT NOT NULL | Defaults to final path component. No rename support in the model despite SPEC §9 calling projects user-facing. Add when the rail gains rename. |
| `created_at` / `last_activity` | INTEGER | Present in both table and `ProjectRecord` (`model.rs:378-381`) — the one table where storage and model agree. |
| `history_json` | TEXT (v4) | `ProjectHistory{accounts, native_sessions}` (`model.rs:357-365`). `merge_history` overwrites the blob (`projects.rs` merge); concurrent backfill + launch `touch` can clobber. Parse-merge-write must hold the storage lock — it does today only because everything holds the one Mutex. |

### 2.4 Model types with no storage (and vice versa)

- `UsageSnapshot.window: Option<QuotaWindow>` (`model.rs:303-306`)
  exists in the model but no quota feed writes it — M3 storage
  (windows per account, not per session) is undesigned. Quota is
  an *account* property; attaching `window` to a session snapshot
  will duplicate it N times and invite inconsistency.
- `InstanceKind::Desktop` (`model.rs:330-332`) has a storage
  value and an `as_str`/`parse` pair but no producer: every
  insert writes `Terminal` (`commands.rs:700,1176`). The type is
  ahead of the supervisor, which has no desktop launch path (§4).
- `accounts.created_at`/`profiles.created_at` are stored but
  unmodelled (above). Pick one direction in v5; do not carry
  both halves.

## 3. Migration strategy: sound skeleton, three missing muscles

### 3.1 What the current strategy gets right

- Forward-only migrations indexed by `user_version`
  (`storage/mod.rs:82,135-155`), each in its own transaction with
  `pragma_update(user_version)` inside the same `tx`
  (`storage/mod.rs:149-152`). A crash mid-migration rolls back.
- Newer-database rejection via `CoreError::SchemaTooNew`
  (`storage/mod.rs:138-143`, `error.rs:19-27`) instead of silent
  downgrade — the correct call for a local-first app.
- Tampered negative `user_version` degrades to re-run-and-fail
  (`storage/mod.rs:144-146`) rather than panicking.

### 3.2 Gaps that will bite at v5+

1. **No table-rebuild migrations.** Adding the missing FK on
   `sessions.project` (§2.3) or a CHECK on `accounts.harness`
   cannot be done with `ALTER TABLE ADD COLUMN`. SQLite requires
   create-new/copy/drop/rename. The current `&[&str]` of bare SQL
   (`storage/mod.rs:82`) can express it, but there is no helper,
   no test that a v3 database upgrades cleanly, and no guidance
   that rebuilds must disable FK enforcement during copy. Decide
   the rebuild pattern *before* the first rebuild is needed.
2. **No migration tests against historical fixtures.** Tests open
   fresh in-memory databases (`open_in_memory` at
   `storage/mod.rs:112-115`). A regression that breaks upgrade
   from a real v1 file would ship unnoticed. Commit one binary
   fixture per released version and upgrade each in CI.
3. **No backup before migration.** SPEC §12 promises one
   `juggler.sqlite` (`docs/SPEC.md:399`) holding lineage and
   usage history, yet `open` migrates in place
   (`storage/mod.rs:99-105`) with no pre-migration copy. F1.7
   (rolling backups + corrupt-DB recovery) is still open. At
   minimum: copy to `juggler.sqlite.bak-v<N>` before any
   migration that rebuilds a table, and keep the last 3.
4. **JSON blobs migrate invisibly.** `settings_json`,
   `usage_json`, `transcript_ref_json`, `history_json` have no
   version field. A serde-incompatible change to
   `SettingsOverlay` (`model.rs:159-168`) would fail at row-map
   time for every profile (`profiles.rs:126`) with no migration
   hook. Embed `"v":1` or migrate blobs lazily on read; choose
   one and write it down.
5. **WAL housekeeping is absent.** WAL is enabled
   (`storage/mod.rs:122`) with `synchronous=NORMAL`, but there
   is no `wal_checkpoint`, no `busy_timeout`, and no second
   connection exists to contend — yet. The moment a read replica
   or the fleet link (§11) opens the file, default `busy_timeout=0`
   turns contention into `SQLITE_BUSY` errors. Set
   `busy_timeout=5000` now, while it is a one-line change.

### 3.3 Proposed v5 (the next migration, concretely)

```sql
-- v5: fix the known debts in one rebuild pass
ALTER TABLE sessions ADD COLUMN status_changed_at INTEGER;
ALTER TABLE sessions ADD COLUMN archived_at INTEGER;
CREATE TABLE usage_events (
  id INTEGER PRIMARY KEY,
  session_id TEXT NOT NULL REFERENCES sessions(id),
  account_id TEXT NOT NULL REFERENCES accounts(id),
  observed_at INTEGER NOT NULL,
  kind TEXT NOT NULL,          -- tokens | quota_window
  payload_json TEXT NOT NULL,
  confidence TEXT NOT NULL     -- measured|computed|estimated
);
CREATE INDEX idx_usage_session ON usage_events(session_id, observed_at);
CREATE TABLE lineage (
  id TEXT PRIMARY KEY,
  source_session TEXT NOT NULL REFERENCES sessions(id),
  target_session TEXT REFERENCES sessions(id),
  kind TEXT NOT NULL,          -- t1|t2|t3|t4
  created_at INTEGER NOT NULL,
  detail_json TEXT NOT NULL DEFAULT '{}'
);
-- sessions PROJECT FK via rebuild (create/copy/drop/rename)
CREATE TABLE machines (
  id TEXT PRIMARY KEY, label TEXT NOT NULL, is_local INTEGER NOT NULL DEFAULT 0
);
INSERT INTO machines VALUES ('this-mac','This Mac',1);
```

This is deliberately boring: it adds what SPEC §6/§7/§12
already promise (`usage_events`, lineage) and nothing more.

## 4. Adapter contract critique: what M2 will break

### 4.1 The trait as built vs SPEC §4 as written

SPEC §4 sketches (`docs/SPEC.md` adapter section) an async trait
with `discover`, `prepare_account`, `launch_spec`, `watcher`,
`usage`, `export`, `prime`. The built trait at
`crates/juggler-adapter/src/adapter.rs:26-124` is synchronous and
implements: `id`, `capabilities`, `verified_against`, `discover`,
`prepare_account`, `launch_spec`, `watcher`, plus defaulted
`scan_environment`, `import_account`, `login_url`, `login_spec`.
The three omissions are the whole handover/usage story:

- **No `usage()`** — `UsageSnapshot` exists in core
  (`model.rs:299-309`) and `USAGE_TOKENS`/`USAGE_QUOTA_FEED` bits
  exist (`capabilities.rs:20-22`), but nothing produces them.
- **No `export()`/`prime()`** — `juggler-handover/src/lib.rs:1-13`
  is doc-only; T3/T4 have no trait home. The Claude adapter
  advertises `TRANSPLANT|FORK` (`claude_code.rs:111-120`) and the
  fake advertises the same (`fake.rs:49-56`), yet the only
  transplant code is the app-level byte-copy in
  `commands.rs:1088-1194`. Advertising a capability with no
  implementing method is the F3.1 honesty bug.
- **No session-binding type.** `LaunchMode::Resume{session_id}`
  (`launch.rs:23-26`) carries a bare `String`; each adapter
  interprets it (Claude: `--resume <id>` at
  `claude_code.rs:294-296`). Codex `exec resume` and ZCode
  `--resume` differ in whether the id is a file stem, a thread
  id, or a DB row id — a stringly contract will leak.

The synchronous design is, however, defensible and should be
kept: adapters do bounded filesystem/process probes, the app
calls them from Tauri commands, and an async trait would drag
`async-trait` boxing into a semver-protected API. SPEC's
`#[async_trait]` sketch should be corrected to match the code,
not the reverse.

### 4.2 Per-harness gaps

**Codex (SPEC §4.2).** Needs `CODEX_HOME` isolation (parallel
to `CLAUDE_CONFIG_DIR` at `claude_code.rs:38`), rollout JSONL
under `sessions/**/rollout-*.jsonl`, `token_usage_record`
parsing, and `codex exec resume <id>`. Blockers in the current
trait: `watcher(transcript_root: &Path)`
(`adapter.rs:68-73`) takes a single root path, which fits
Codex, but `SessionEvent` (`watcher.rs:10-30`) has only
`Discovered`/`MessageAppended`/`Ended` — Codex's app-server
(SPEC §4.2 phase 2, `PROTOCOL_SERVER` bit at
`capabilities.rs:24`) emits approvals and turn lifecycle that
have no variant. Also `verified_against: &[&str]`
(`adapter.rs:35`) is a static list; Codex's fast churn needs
range expressions. Minimum viable Codex work is PTY + exec
mode; do not design the trait around app-server yet.

**ZCode (SPEC §4.3).** The hardest fit. Sessions live in a
vendor SQLite database (`<home>/.zcode/cli/db/db.sqlite`),
isolation is full `HOME` redirect, launch is `node zcode.cjs`,
and transplant is schema-coupled row cloning (SPEC T2 notes) —
explicitly unverified. The trait's `watcher(&Path)` assumption
(a directory of transcript files) does not hold: ZCode needs a
`DbWatcher` that opens the vendor DB read-only. Forcing ZCode
through `transcript_root: &Path` will produce a lying adapter.
ZCode should ship T1/T3 only, with `TRANSPLANT` unset, until
the row-clone is fixture-proven.

**Desktop-class harnesses (SPEC §9, ADR-0007).**
`InstanceKind::Desktop` (`model.rs:330-332`) promises windowed
apps with "no terminal surface", but `LaunchSpec`
(`launch.rs:32-43`) has only `program/args/env/cwd/needs_pty`
and the supervisor has exactly two spawn paths (PTY, piped) at
`supervisor.rs:107-111`. A desktop harness needs focus/raise,
window identity, and "is the app already running?" semantics.
None exist. Do not add `Desktop` producers until the supervisor
gains a `LaunchKind`.

### 4.3 Proposed trait changes (exact signatures)

Keep the trait synchronous and object-safe. Add four methods
with default implementations so existing adapters keep
compiling, and downgrade `TRANSPLANT` advertising until
`transplant` exists and is fixture-tested:

```rust
// crates/juggler-adapter/src/adapter.rs — proposed additions
pub trait HarnessAdapter: Send + Sync {
    // ... existing methods unchanged ...

    /// Current usage/quota for one account (read-only, no network).
    fn usage(&self, account: &Account)
        -> Result<UsageReport, AdapterError> {
        Err(AdapterError::Unsupported("no usage feed".into()))
    }

    /// Exports a native session to the neutral Transcript IR.
    fn export_session(&self, transcript: &TranscriptRef)
        -> Result<Transcript, AdapterError> {
        Err(AdapterError::Unsupported("no transcript export".into()))
    }

    /// Prepares a transplant of `source` into `target` state dir.
    /// Returns the native id to resume under the target account.
    /// Implementations must be copy-only (source never mutated).
    fn transplant_session(
        &self,
        source: &TranscriptRef,
        target_state_dir: &Path,
        target_cwd: &Path,
    ) -> Result<TransplantOutcome, AdapterError> {
        Err(AdapterError::Unsupported("no transplant".into()))
    }

    /// Builds the launch spec that primes a new session from a
    /// context pack / exported transcript (T3/T4 handover-in).
    fn prime_spec(
        &self,
        account: &Account,
        profile: &Profile,
        cwd: &Path,
        pack: &ContextPackRef,
    ) -> Result<LaunchSpec, AdapterError> {
        Err(AdapterError::Unsupported("no priming".into()))
    }
}
```

Supporting types to add:

```rust
// crates/juggler-adapter/src/usage.rs (new)
pub struct UsageReport {
    pub account: AccountId,
    pub windows: Vec<QuotaWindowState>, // label, used_pct, resets_at
    pub confidence: Confidence,         // from juggler-core
    pub observed_at: Timestamp,
}
pub struct QuotaWindowState {
    pub label: String,                  // "5h" | "weekly" | "credits"
    pub window: QuotaWindow,
}

// crates/juggler-adapter/src/handover.rs (new)
pub struct TransplantOutcome {
    pub native_id: String,              // id valid under target
    pub files_copied: u64,
    pub bytes_copied: u64,
    pub forked: bool,                   // collision resolved by fork
}
pub struct ContextPackRef {
    pub pack_dir: PathBuf,              // managed packs/ dir
    pub source_native_id: Option<String>,
}
```

And in `launch.rs`, replace the stringly resume id with a
typed binding without breaking current implementors:

```rust
pub enum LaunchMode {
    Interactive,
    Headless { prompt: String },
    Resume { binding: SessionBinding },
}
pub enum SessionBinding {
    NativeId(String),                   // today's behaviour
    TranscriptPath(PathBuf),            // Claude transcript-path resume
}
```

Migration path: Claude implements `transplant_session` by
moving `commands.rs:1120-1146` logic into the adapter (using
`TranscriptRef.path`, not newest-by-mtime), implements
`export_session` via the existing parser
(`claude_transcript.rs` `parse_transcript`/`dedupe_messages`),
and only then re-advertises `TRANSPLANT`. The fake harness
implements all four first — it is the conformance kit (F1.8) —
so every future adapter is tested against the same fixtures.
ZCode leaves `transplant_session` defaulted (Unsupported) and
does not set `TRANSPLANT`; Codex implements `NativeId` resume
plus `TranscriptPath` if `exec resume` accepts it, verified by
fixture, not assumed.

## 5. Supervisor: failure modes (FMEA)

Supervisor core: `crates/juggler-supervisor/src/supervisor.rs:52-61`
(`children: Mutex<HashMap>`, `user_stopped: Mutex<Vec>`, one
`mpsc` event channel), journal `journal.rs:26-30`, ring
`ring.rs:11-17` (512 KiB default, `supervisor.rs:19-20`).
Stop ladder: `supervisor.rs:296-318` (SIGTERM, 10 ms poll,
force-kill). Spawn paths: piped `supervisor.rs:122-178`
(2 readers + 1 waiter), PTY `supervisor.rs:180-266`
(1 reader + 1 waiter). Dropping the supervisor intentionally
does not signal children (`supervisor.rs:47-51`).

| # | Failure | Current behaviour (evidence) | Effect | Fix |
|---|---|---|---|---|
| F-1 | App crash, children survive | Journal `Spawned` without `Exited` → adopted (`journal.rs:151-196`); reconciler marks non-adopted Started/Running `Stopped` (`state.rs:56-72`) | Correct by design; adopted sessions have no PTY/ring (see F-3) | Keep; surface "adopted, terminal detached" state |
| F-2 | PID reuse after crash | Adoption checks only PID liveness via `sysinfo` (`journal.rs:188-196,199-216`); journal stores `program` (`journal.rs:35-40`) but never compares it | A recycled PID owned by an unrelated process is adopted as a live session; `stop` then SIGTERMs a stranger (`supervisor.rs:309`) | Verify `/proc`/`ps` exe path + start-time vs journal; store start-time in journal |
| F-3 | Adopted orphan control | `alive_adopted_sessions` returns ids only (`supervisor.rs:432-438`); no `ChildHandle` is rebuilt | UI shows Running with no output/input/stop path — `stop` returns `UnknownSession` (`supervisor.rs:296-303`) | Rebuild handle in adopt mode (output unavailable, stop = kill PID) or mark `Detached` |
| F-4 | PTY reader EOF vs process exit | Reader breaks on `Ok(0)/Err` (`supervisor.rs:492-504`); exit comes only from waiter thread (`supervisor.rs:242-259`) | If waiter `wait()` fails, `finished` set silently (`supervisor.rs:256-258`), no `Exited` event, session stuck Running until restart | Emit synthetic `Exited{Code(-1)}` on waiter failure |
| F-5 | Zombie accumulation | Children are waited (`supervisor.rs:158,243`); adopted orphans are *not* our children, `alive_pids` excludes Zombie/Dead (`journal.rs:208-213`) | Our zombies: none. Orphan zombies correctly counted dead. Residual: piped grandchildren reparented to init — out of scope, document it | None; keep zombie filter |
| F-6 | PTY/handle leak on stop | `stop` kills but never removes the `ChildHandle` (`supervisor.rs:285-287` insert-only; no `remove` anywhere) | `children` map + 512 KiB rings grow for process lifetime; 50 stopped sessions retain ~25 MiB (see §6) | Remove handle (or shrink ring) on `Exited` after snapshot retention window |
| F-7 | Disk full | Journal `append` opens+writes per record (`journal.rs:82-98`); spawn ignores journal error only at `record_exited` (`supervisor.rs:517`, `let _ =`), `spawn` propagates (`supervisor.rs:112-114`); SQLite WAL grows unchecked | Full disk → spawn fails *after* process started (spawn precedes journal at `supervisor.rs:107-114`), orphan with no record | Journal-before-spawn intent record; check free disk (SPEC §5.2) at admission |
| F-8 | DB lock / poisoned Mutex | `AppState.storage: Mutex<Storage>` (`state.rs:17`); poison → `AppError::StatePoisoned` (`commands.rs:830-838`); lifecycle forwarder holds lock per event (`lib.rs:60-68`) | One panic while holding the lock poisons *all* storage access until restart; no recovery path | `lock().unwrap_or_else(into_inner)` + log; longer term, split read pool (§7) |
| F-9 | Descendant leak on kill | Force-kill kills only the direct child (`supervisor.rs:149-151,234-236`); `descendants` (`supervisor.rs:347-356`) is query-only, never used by `stop` | Claude's spawned subagents/tools survive a "stopped" session and keep burning quota | Kill the process group/tree in the ladder (TERM tree → grace → KILL tree) |
| F-10 | Event channel stall | Single consumer polls 500 ms (`lib.rs:57-62`); `events_tx.send` ignores failure (`supervisor.rs:115-118,519-523`) | Bursty exits queue unboundedly in `mpsc`; dashboard lags but does not crash. Acceptable; bound or document | Document; add channel-depth metric at 20+ sessions |

Two invariants hold and must not regress: stop intent is
tracked explicitly (`user_stopped`, `supervisor.rs:55-58,305-308`)
because PTY kills surface as non-zero codes, and exit
classification (`lifecycle.rs:9-18`, `commands.rs:793-804`)
maps user-stop → `Stopped`, never `Failed`. The `Signaled(_) →
Stopped` mapping at `commands.rs:801` is, however, too broad:
an OOM-killed (SIGKILL) session also becomes `Stopped`,
hiding crashes. Distinguish SIGKILL+memory-pressure (SPEC
§5.3) from user-initiated SIGTERM.

## 6. Scale model: 10 accounts / 50 sessions

### 6.1 Thread and memory accounting (per live session)

| Resource | PTY session | Piped session | Evidence |
|---|---|---|---|
| Reader threads | 1 | 2 (stdout+stderr) | `supervisor.rs:222-224` vs `143-144` |
| Waiter threads | 1 | 1 | `supervisor.rs:242`, `157` |
| Terminal forwarder | 1 (when attached) | 0 | `terminal.rs:100-102` |
| Ring memory | 512 KiB retained | 512 KiB | `supervisor.rs:20`, `ring.rs:22-30` |
| Watcher | shared per account (notify + rescan) | same | `claude_code.rs:418-436` |

At 50 live PTY sessions: 100 supervisor threads + up to 50
forwarder threads + Tauri/tokio runtime. Thread count is not
the binding constraint (each blocks on I/O); ring memory is
50 × 512 KiB = **25 MiB retained**, plus per-push byte-by-byte
`VecDeque` churn in `ring.rs:42-47` (push iterates per byte —
a 1 MiB burst is 1M deque ops; rewrite `push` with extend/
drain before 50-session dogfood). At 10 accounts: 10 watchers,
each holding an `notify` backend handle — fine on macOS
(FSEvents coalesces), but each `poll` timeout triggers a full
rescan (below).

### 6.2 The three superlinear costs

1. **Transcript rescan.** `scan()` at `claude_code.rs:453-473`
   walks the whole `projects/` tree, `read`s **every** `.jsonl`
   fully (`claude_code.rs:458`) and counts lines, on every
   poll cycle. Cost is O(total transcript bytes) per watcher
   per poll. With 10 accounts × ~200 MB of transcripts this is
   gigabytes of re-reads per minute. Fix (F5.1): persist
   `(path, offset, mtime)` per file; parse appends only.
2. **SQLite serialization + N+1.** Every command locks the
   one Mutex (`state.rs:17`). `session_infos`
   (`commands.rs:911-961`) runs accounts list + per-account
   profile lists + sessions list + two more account lists
   (accounts listed **three times**: `commands.rs:912-917`,
   `931-936`, plus profiles loop) — O(accounts + profiles +
   sessions) queries per board refresh, and `emit_sessions`
   (`commands.rs:817-825`) re-runs it on every lifecycle event.
   At 50 sessions this is still milliseconds, but it holds the
   global lock during all of it, stalling launches. Fix: one
   JOIN query + cache account/profile maps; measure before
   adding a connection pool (WAL allows concurrent readers).
3. **`sysinfo` full refreshes.** `descendants_of`
   (`supervisor.rs:557-579`) and `alive_pids`
   (`journal.rs:200-216`) each construct `System::new()` and
   refresh **all** processes per call. Called per adoption
   check / per descendant query, this is O(processes) with
   allocation churn. Fix: one shared `System` refreshed on a
   2 s interval (SPEC §5.2 staggered refresh), reused by both.

### 6.3 Projected behaviour at 10/50 (estimates, labelled)

Assuming the fixes above are *not* made: launch latency stays
flat (spawn is per-session), board refresh degrades linearly
with stored sessions (no pagination in `sessions list` at
`sessions.rs:125-145` — every refresh materializes all rows),
and watcher CPU becomes the dominant background cost. With
the fixes: 50 sessions is comfortably within one process;
the next wall is the webview (50 xterm instances), which is
a UI virtualization question, not a Rust one. Add the
20-session fake-harness load test (F5.1) before claiming
either way — today there is no load evidence at all.

## 7. Security & compliance audit (invariant by invariant)

Invariants: proposal §5 / SPEC §8 (`docs/SPEC.md` Security:
credentials never read/copied/stored; managed dirs 0700; local only).

| Invariant | Verdict | Evidence |
|---|---|---|
| No credential reads | **Holds** | `login_state` metadata-only (`claude_code.rs:332-344`); keychain probe omits `-w` (`claude_code.rs:579-589`) |
| No credential copies | **Violated (F4.1)** | Import byte-copies `.credentials.json` (`claude_environment.rs:209-232`, perms tightened `228-230`) vs SPEC §8. Adopt prior option (b): amend SPEC to "copied only into 0700 managed dir on user-initiated import", plus guardrail test |
| Writes confined to managed dirs | **Holds** | `prepare_managed_dir` 0700+marker (`claude_code.rs:389-403,538-555`); `atomic_write` (`core/fs.rs:18-30`); import dest-must-not-exist (`claude_environment.rs:180-185`), source read-only |
| Passive / click-bound | **Holds, unenforced** | Mutations behind Tauri commands (`lib.rs` handler list); no proxy/`BASE_URL` code. But `tests/guardrails.rs` (SPEC §7) does not exist — write it before M2 (F4.2) |
| Vendor login untouched | **Holds** | Guided terminal only (`claude_code.rs:354-372`, `terminal.rs:56-73`) |
| Env secrets at rest | **New finding** | `SettingsOverlay.env` (`model.rs:165-167`) persists plaintext in `settings_json` (`profiles.rs:40`) inside `juggler.sqlite`; launching also merges full parent env into PTY (`supervisor.rs:201-206`). Redact env values in diagnostics/logs; document that profile env is secret-bearing |
| Spawn integrity | **Watch** | Binary resolved via PATH + fallbacks (`claude_code.rs:64-81`) plus env override `AI_JUGGLER_CLAUDE_BIN` (`claude_code.rs:41,68-73`) with only `is_file` checked. A writable PATH dir hijacks launches. Prefer absolute resolution + version probe (already in `discover`, `claude_code.rs:126-194`) reused at launch |

## 8. Target architecture & ADRs to write

**Target (evolution, not rewrite):** keep the 7-crate layout; move transplant/export into `juggler-handover` behind the new trait methods (§4.3); add `usage_events`+`lineage` (v5, §3.3); WAL read pool only if measured; incremental transcript tailing with persisted offsets; shared `sysinfo` refresher; supervisor gains adopt-handles, tree-kill, journal start-time; `juggler-link` stays stubbed until M6.

**ADRs to write (in order):** ADR-0008 adapter trait v2 (usage/export/transplant/prime, sync kept); ADR-0009 usage & lineage storage (v5 schema + blob versioning); ADR-0010 supervisor adoption identity (PID+exe+start-time, tree-kill); ADR-0011 credential-import carve-out (SPEC §8 amendment); ADR-0012 watcher tailing & resource-manager refresh budget; ADR-0013 desktop-harness `LaunchKind` (deferred until needed).

**Unresolved evidence:** no load test at 10/50 exists; ZCode DB schema and Codex `exec resume` path semantics unverified against real binaries; desktop-harness requirements are SPEC-only.
