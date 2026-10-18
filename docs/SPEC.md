# AI Juggler — Technical Specification

**Status:** draft v0.2 for review (revised after first review round) · **Date:** 2026-10-12
Companion: [proposal](./PROPOSAL.md) · Evidence: [research notes](./research/)

This document specifies the architecture and product behavior of AI Juggler v0.x. Everything here is grounded in the verified findings in `docs/research/`; where a mechanism is version-fragile, the research note is cited.

---

## 1. Overview & constraints

AI Juggler is a native desktop application that **supervises** AI coding harnesses the user already has installed. It never proxies model traffic, never automates vendor logins, and never stores vendor credentials (see proposal §5 guardrails — these are product-level invariants, enforced in code review and tests).

**Non-functional requirements**

| Requirement | Target |
|---|---|
| Platform (v0.x) | macOS 13+ (Universal: Apple Silicon + Intel). Windows/Linux post-1.0; the Rust core and adapters are cross-platform from day one (CI builds all three OSes) so ports need no rewrites |
| Startup | < 1.5s cold start; session board interactive < 500ms after launch |
| Footprint | < 150 MB RSS typical; UI at 60 fps with 20 live sessions |
| Reliability | App crash never kills supervised sessions (children survive; state journaled); recovery on relaunch |
| Resilience | All writes atomic (temp+rename); SQLite in WAL mode; Provider-format parse failures degrade, never crash |
| Privacy | No network calls except: update feed check, and user-initiated actions in managed Providers. Telemetry opt-in only |
| Resource monitoring | < 1% CPU overhead; launch-advisor projections within ±20% of observed session peaks after calibration |

**Automation policy (product invariant).** The app *prepares*; the user *executes*. Automated behavior is strictly passive — monitoring, indexing, usage accounting, notifications, alerts, suggestions, and staging of actions. Every state-changing action (switching an account for new work, transplanting a session, priming a target, syncing project context) is bound to an explicit user click; there is no code path that performs one without it (asserted in `tests/guardrails.rs`). One deliberate, narrowly-scoped exception: an opt-in, per-Launch-settings *auto-restart on crash* (off by default) that relaunches a crashed process into the **same** recipe — it never changes accounts, conversation content, or anything else user-authored. Two further recorded amendments exist, no others may be invented: (1) **verified cleanup** — opt-in per-Account automatic kill of ledger-verified leftover processes only, audit-logged (ADR-0010, §5.2); (2) **pack secret policy** — the user's remembered redaction choice governs pack contents (ADR-0009, §3.3); it never silently decides, and is always shown before a switch executes.

## 2. System architecture

```mermaid
flowchart LR
    subgraph UI["Tauri shell (webview UI)"]
        FE[Dashboard · Profiles · Session Board · Usage · Handover]
        TERM[Embedded terminals<br/>xterm.js]
    end
    subgraph CORE["Rust core (tokio)"]
        API[command/event API]
        REG[Adapter registry]
        SUP[Process supervisor<br/>portable-pty]
        RM[Resource manager<br/>sysinfo · footprints]
        IDX[(Session indexer<br/>notify + parsers)]
        USE[Usage engine]
        HO[Handover engine]
        DB[(SQLite<br/>rusqlite, WAL)]
        KEY[OS keychain<br/>keyring]
    end
    subgraph PROVIDERS["Managed Providers (user-installed, unmodified)"]
        CC[Claude Code<br/>CLAUDE_CONFIG_DIR=...]
        CX[Codex<br/>CODEX_HOME=...]
        ZC[ZCode<br/>HOME=...]
    end
    FE <--> API
    TERM <--> SUP
    API --> REG --> SUP
    SUP <--> RM
    SUP -->|spawn| CC & CX & ZC
    IDX -->|watch & parse| CC & CX & ZC
    USE --> IDX
    HO --> IDX
    CORE --> DB & KEY
    CORE -.-|gRPC · mTLS · mDNS| PEERS[Peer AI Juggler<br/>instances (fleet)]
```

**Layering rules**

- The UI is a thin view; all state lives in the Rust core and is streamed to the UI via Tauri events (pre-aggregated payloads — the webview never parses raw transcripts).
- Only adapters know Provider specifics. Core speaks only in domain types.
- Everything on disk is append-only or atomic; nothing in a harness's own directories is ever written except through adapter-defined, user-visible actions (e.g., creating an isolated profile directory).

### Workspace layout (Cargo)

```
ai-juggler/
├── crates/
│   ├── juggler-core/        # domain types, events, storage (SQLite), usage math
│   ├── juggler-adapter/     # HarnessAdapter trait, capabilities, IR schemas
│   ├── juggler-adapters/    # claude-code, codex, zcode implementations
│   ├── juggler-supervisor/  # PTY/process lifecycle, health, stuck detection, resources
│   ├── juggler-handover/    # context packs, rules-file sync, takeover flows
│   ├── juggler-link/        # fleet: mDNS discovery, pairing, gRPC/mTLS sync
│   └── juggler-app/         # Tauri shell, commands/events, updater wiring
├── ui/                      # frontend (React + TypeScript)
└── docs/
```

## 3. Domain model (ADR-0007, ADR-0008; terminology charter)

<!-- Legacy→canonical glossary (ADR-0008; linked from PROPOSAL/EPICS) -->

| Legacy term (≤ v0.0.x) | Canonical term | Since |
|---|---|---|
| harness | **Provider** | schema v5 / AJ-E102-107 |
| legacy profile word | **Account** when it meant the login; **Launch settings** otherwise | D-10 |
| profile (as launch config) | **Launch settings** (`LaunchRecipe` internal) | D-10 |
| session (as running process) | **Instance** | ADR-0007/0008 |
| session (conversation) | **Session/Conversation** — unchanged meaning | — |
| handover | **Switch** (unlimited, bidirectional) | D-01 |

Vocabulary (user-facing and normative in this spec): a **Provider** is a
tool family you subscribe to (Claude Code, Codex, ZCode, later Gemini,
OpenCode…); an **Account** is one subscription/login under a Provider; an
**Instance** is one running occurrence of a tool under one Account on one
project; a **Session** is the conversation inside an Instance. "Profile"
is demoted to **launch settings** — a per-Account overlay used at launch;
it is never a rail noun and never means Account (D-02/D-10).

```rust
// crates/juggler-core/src/model.rs (sketch; aliases noted are the
// migration path — wire values never change)

pub struct HarnessId(Arc<str>);          // "claude-code" | "codex" | "zcode"
pub type ProviderId = HarnessId;         // ADR-0008 alias (additive)

pub struct Account {
    pub id: AccountId,
    pub harness: HarnessId,              // UI: "Provider"
    pub label: String,                   // "Work Max", "Personal"
    pub plan: Option<PlanTier>,
    pub state_dir: PathBuf,              // the isolated dir AI Juggler manages
}

pub struct Profile {                     // UI noun: "Launch settings"
    pub id: ProfileId,
    pub account: AccountId,              // ALWAYS Account-scoped (D-10)
    pub settings_overlay: SettingsOverlay, // model, permissions, mcp, env
    pub default_cwd: Option<PathBuf>,
}

pub struct SessionRecord {               // domain alias target: InstanceRecord
    pub id: SessionId,                   // juggler's instance id (AJ-E104)
    pub kind: InstanceKind,              // Terminal | Desktop (desktop = windowed
                                         // desktop Provider: no embedding, Focus Window only)
    pub native_id: Option<String>,       // the conversation id inside the instance
    pub profile: ProfileId,
    pub machine: MachineId,              // fleet: origin machine (default this Mac)
    pub project: Option<ProjectId>,      // the juggling anchor (canonical root)
    pub cwd: PathBuf,
    pub status: SessionStatus,           // Starting|Running|WaitingUser|Stuck|
                                         // QuotaBlocked|Finished|Stopped|Failed|HandedOver
    pub started_at: Timestamp,
    pub last_activity: Option<Timestamp>,
    pub usage: UsageSnapshot,
    pub transcript_ref: Option<TranscriptRef>,
}

pub struct ProjectRecord {               // the juggling anchor (ADR-0007)
    pub id: ProjectId,
    pub root: PathBuf,                   // canonical
    pub name: String,
    pub history: ProjectHistory,         // imported pre-Juggler account history
}
```

### 3.1 Switch Graph lineage (ADR-0011; supersedes linear handover chains)

Unlimited bidirectional switching needs a graph, not a chain:

- **Work item** — the user's continuing task on one Project. It survives
  every switch; Instances come and go underneath it.
- **Node (instance-epoch)** — one Instance running under one Account from
  launch/resume until it stops, freezes, or hands off. Resuming the same
  native conversation after switch-back creates a *new epoch node
  pointing at the same `native_id`* — that pointer is what makes
  switch-back a resume, not a copy (D-05).
- **Edge (`SwitchEdge`)** — one executed switch: `id, work_id,
  from_epoch, to_epoch, from/to_account, from/to_provider, mechanism
  (native-resume | transplant | replay | pack), pack_id, fidelity_shown,
  artifact_manifest_sha256, state, created_at, actor`.

Invariants: **S-G1** edges are append-only — failed/undone switches add
state, never rewrite history (undo = a compensating resume edge);
**S-G2** the graph is per-Project — cross-project moves are exports (T4
packs), not edges, because `cwd` is baked into every native format;
**S-G3** message-uuid dedupe is cycle-safe (first-wins per uuid), so
A→B→A never duplicates messages.

Every lineage query (project history, "continue on", divergence
detection) reads this graph. Fields the engine cannot yet populate
render `Unavailable — <reason>`, never a guess (D-06).

### 3.2 Spawn ledger (ADR-0010)

Every process AI Juggler spawns is recorded with intent **before** the
spawn: ledger row = `{pid, pgid, machine, instance, provider, account,
argv hash, cwd, started_at, exit, reaped}`. The ledger — not ppid
walking — is the ownership authority for cleanup (§5): a process not in
the ledger is never touched. Re-adoption after app restart reconciles
the ledger against live processes (zombies-in-the-ledger are dead rows).

### 3.3 Portable context pack v2 (ADR-0009)

The canonical staging/preview artifact for every switch and export:

```
pack/
├── PACK.md              # task, constraints, decisions, open TODOs, next steps
├── conversation.md      # as carried (tier-dependent, see §7)
├── state.json           # repo, branch, diff summary, env, providers,
│                        # conversation ids, Switch Graph lineage embedding
├── manifest.json        # redaction manifest: every secret-shaped artifact
│                        # detected, and how the active policy treated it (D-14)
└── diffs/               # key patches if not committed
```

Pack generation is deterministic from the Transcript IR. Secret handling
is the **user's remembered choice** (global policy + per-Provider
override, visible before any switch executes — never a hidden default;
D-14).

### 3.4 Honesty states

Anywhere a value exists in the model, an absent value is explicit:
`Unknown — no renewal date entered` (D-12), `Unavailable — Provider does
not expose <signal>`, `Unvalidated — beta pair (see §7)`. Composite
"quality scores" do not exist in the domain (D-07).

### 3.5 Migration posture

All renames in this section are **additive aliases** implemented in
E1: `ProviderId`/`InstanceRecord` type aliases with serialisation
identical to today; wire values (`claude-code`/`codex`/`zcode`, status
strings) unchanged; existing DBs and exported JSON round-trip. No
big-bang rename (D-02).

### 3.6 Done checklist (§3 rewrite, AJ-E901)

- [x] Old noun "harness" removed as a user-facing term (code name
      `HarnessId` retained as alias; §4 title updated to "Provider
      adapters" where user-facing)
- [x] Provider / Account / Instance / Session(launched-conversation) /
      Launch settings defined with code mapping
- [x] Work / Epoch / SwitchEdge + invariants S-G1..S-G3 added
- [x] Spawn ledger entity added (ownership authority)
- [x] Pack v2 with lineage embedding + D-14 redaction manifest
- [x] Honesty states (Unknown/Unavailable/Unvalidated) normative
- [x] Cross-refs updated: §5 (ledger), §6 (two clocks), §7 (Switching),
      §9 (Inspector nouns), §11 (MachineId on every row)

## 4. Adapter system

The adapter is the extensibility contract. First-class Rust adapters ship for the big three; post-v1, a declarative manifest (TOML) covers simple CLIs without recompiling.

```rust
// crates/juggler-adapter/src/lib.rs (sketch)

bitflags! {
    pub struct Capabilities: u32 {
        const HEADLESS | RESUME | TRANSCRIPT_EXPORT | TRANSCRIPT_WATCH
            | USAGE_TOKENS | USAGE_QUOTA_FEED | PROTOCOL_SERVER | FORK = 0;
    }
}

#[async_trait]
pub trait HarnessAdapter: Send + Sync {
    fn id(&self) -> HarnessId;
    fn capabilities(&self) -> Capabilities;

    /// Detect install, version, and drift vs. versions this adapter was verified against.
    async fn discover(&self) -> DiscoveryReport;

    /// Prepare an isolated account state dir (idempotent) and return launch env.
    async fn prepare_account(&self, dir: &Path) -> Result<EnvMap>;

    /// Build the launch spec for a profile in a given cwd.
    async fn launch_spec(&self, profile: &Profile, cwd: &Path, mode: LaunchMode)
        -> Result<LaunchSpec>;               // argv + env + tty needs

    /// Watch/index native sessions (read-only!).
    fn watcher(&self) -> Box<dyn SessionWatcher>;

    /// Extract usage signals; never performs network calls.
    async fn usage(&self, account: &Account) -> UsageReport;

    /// Export a session to the neutral Transcript IR.
    async fn export(&self, transcript: &TranscriptRef) -> Result<Transcript>;

    /// Prime a new session from a ContextPack (handover in).
    async fn prime(&self, pack: &ContextPack, spec: LaunchSpec) -> LaunchSpec;
}
```

**Design rules**

- Adapters are *read-only* toward native state except for their own managed dirs, and `prepare_account` is the only writer.
- Every parser is versioned: adapters declare `verified_against: ["2.1.296"]`-style ranges; `discover()` warns on drift and capability-probes at runtime (never assumes).
- Session IR is a versioned, neutral schema (`Transcript { meta, messages[], usage[] }`) with per-Provider converters — this is what switching and the indexer share.

### 4.1 Adapter: Claude Code (reference implementation)

- **Launch:** real `claude` binary; interactive via PTY, or headless `-p --output-format stream-json` for driven runs. Env: `CLAUDE_CONFIG_DIR=<managed>`.
- **Sessions:** watch `<config_dir>/projects/**/*.jsonl` (notify debouncer); parse lines (`uuid/parentUuid/type/message/usage`) with fixture-tested, version-pinned parser; dedupe by message `uuid` across resumed forks (ccusage pattern).
- **Usage/quota (per research §5):**
  - *Quota feed:* install a managed statusline script per account dir; its stdin JSON carries `rate_limits.five_hour/seven_day.used_percentage/resets_at` for Pro/Max — captured to the usage engine as **Measured**.
  - *Tokens:* transcript `usage` objects per assistant message → **Measured**.
  - *Exhaustion:* `system/api_retry` rate_limit events (headless), `StopFailure` hook, and transcript pattern matching for "hit your session/weekly limit" (installed as a managed hook in account dirs) → session status transitions + notifications.
- **Resume:** `claude --resume <id>` / `--continue`; transcript-path resume for cross-dir portability.
- **Prime (handover-in):** launch with the context pack as initial user turn (headless or interactive), or `--settings` overlay for per-session config.

### 4.2 Adapter: Codex

- **Launch:** `codex` with `CODEX_HOME=<managed>`; recipes via adapter-generated `<name>.config.toml` (post-0.134 semantics — separate files, not legacy tables).
- **Control plane (phase 2):** `codex app-server` JSON-RPC (thread/turn lifecycle, approvals) — schemas generated via `codex app-server generate-json-schema` and vendored; phase 1 uses PTY + `codex exec --json` for driven runs.
- **Sessions:** watch `~/.codex… <home>/sessions/**/rollout-*.jsonl`; index from `session_meta` without full replay; `token_usage_record` lines → **Measured** tokens.
- **Usage/quota:** no public API (research §5) — token burn is Measured; plan-credit correlation is **Estimated** (window reconstruction from token timestamps; calibrated as the user observes real resets) and labeled as such in the UI.
- **Resume:** `codex resume` (interactive) / `codex exec resume <id>`.

### 4.3 Adapter: ZCode

- **Launch:** bundled engine at `/Applications/ZCode.app/Contents/Resources/glm/zcode.cjs` via `node`, with `HOME=<managed>`; `--surface terminal`, `-p ... --json` headless; version pinned to the installed app version.
- **Sessions:** read-only queries against `<home>/.zcode/cli/db/db.sqlite` (session/message/model_usage tables) + file watching; WAL mode makes concurrent reads safe.
- **Usage/quota:** **Computed exactly** — `model_usage` token counts + the published credit formula (per-model multipliers, off-peak 50% windows) → exact credit burn per session/account; remote pool totals (5h/weekly) remain as configured plan caps since no public API exists.
- **Resume:** `--resume <sess_id>` / `-c`.
- **Constraints honored:** no reverse-engineered endpoints, no patched binaries — documented/observable surfaces only (research A3/B7).

## 5. Process supervisor, spawn ledger & verified cleanup (ADR-0010)

### 5.1 Supervision

- One task per Instance. Spawn via `portable-pty` (real TTY) with
  per-launch-settings env; optional direct child (no PTY) for headless
  runs. Every spawn writes its **intent to the spawn ledger before the
  process exists** — a crash between ledger-write and spawn yields a
  dead row, never a phantom owner (§3.2).
- Output fan-out: (1) UI terminal (xterm.js via Tauri events,
  back-pressured ring buffer), (2) heuristic engine, (3) adapter stream
  parser for structured events.
- **Stuck detection heuristics** (thresholds user-tunable): no-output +
  low CPU + stale transcript mtime → *Stuck?*; alive but awaiting
  permission → *WaitingUser*; non-zero exit with rate-limit signature →
  *QuotaBlocked* (feeds the switch *suggestion*, never an action).
- Lifecycle: start, stop (SIGTERM→SIGKILL ladder; no SIGSTOP ever —
  §1), attach/detach. Children survive app death by design; re-adoption
  on restart reconciles the ledger against live processes.

### 5.2 Ownership and verified cleanup (D-08 — a recorded, narrow
amendment to the passive-automation invariant; ADR-0010)

The maintainer-verified failure mode: an Instance exits, and the dev
servers, MCP transports, watchers and language servers it spawned keep
running for days holding ports and RAM. AI Juggler owns exactly what it
spawned — and nothing else.

- **Ownership authority = the spawn ledger** (§3.2), with process-group
  id (Unix PGID) captured at spawn where available; Windows gets a
  documented seam (Job objects, E5). A process that cannot be verified
  as ledger-owned is never touched — **fail closed** (AJ-E503).
- **ProcessIdentity verify-before-kill:** at click time, re-verify pid +
  pgid + start-time + argv hash against the ledger row; show the user
  the exact command line, cwd, and held ports; only then offer the
  staged TERM → grace → KILL ladder, with per-process outcomes and a
  port re-probe after.
- **Tiers (per Account, not per Provider — D-08; the operating rules
  are ADR-0010's E508 amendment):**
  - **Tier 0 — Observe (default everywhere):** leftovers surface in
    the Cleanup Centre with evidence; nothing is ever killed
    automatically.
  - **Tier 1 — Tidy verified orphans (opt-in per Account, off by
    default):** after `orphan_grace` and a same-tick identity
    re-verification, TERM→KILL via the staged ladder — ONLY helpers
    whose root exited cleanly (code 0) and whose role guess is
    `DevServer|Watcher`. Audit intent before every signal.
  - **Tier 2 — Tidy after crash (a separate per-Account opt-in, off by
    default):** as Tier 1 but for crash-exited roots, only after
    5 minutes AND one user-visible notification with an Undo window
    (≥30 s) before any signal.
  - **Tier 3 — Hog shed: no automatic implementation** — alert plus
    the staged sheet only; widening requires a new decision entry.
  - **Hard exclusions at every tier:** roots; `McpServer`,
    `TerminalShell`, and any database-role guess; re-adopted
    survivors; unverifiable rows; escaped-group helpers; non-opted-in
    Accounts; other machines' processes. A failed same-tick
    re-verification demotes the candidate to Tier 0 (list-only) with
    an audit note. A global `Pause automatic cleanup` kill-switch
    takes effect within one tick; Tiers 1-2 are suspended for 24 h
    after a crash recovery until re-confirmed.
  - Manual cleanup (the verify-before-kill sheet) is always available
    at Tier 0 and above, and is itself always audit-logged.
- **Orphan detection:** post-close sweep of ledger rows vs live
  process groups, with held-port attribution, target ≤10 s after an
  Instance closes (budget for E5, not a current measurement).
- **Fleet shape (D-13):** every ledger row carries `MachineId`;
  ownership, verification, and audit are per-machine, and a remote
  machine's ledger is never acted on without its own permission level
  (§11).
- **Preserved from before:** no SIGSTOP; the stop ladder unchanged;
  unowned processes are untouchable in every tier.

### 5.3 Resource manager (unchanged scope; budgets for E4)

Monitored signals, footprint history, the pre-launch admission advisor
("OK / Tight / No — and why"), overload alerts, and the recommendation
engine remain as previously specified (sysinfo-based, staggered
refresh, per-session tree attribution via the ledger rather than
ppid-guessing). Inspector aggregation budget: ≤200 ms at 50 Instances.

### 5.4 Crash recovery

As before (native-resume offer, relaunch, or switch; opt-in
per-launch-settings auto-restart into the *same* account; re-adoption
rebuilds from ledger + journal). Recovery actions remain user-clicked.

## 6. Usage engine — two clocks (ADR-0012; D-12)

Two distinct clocks, never conflated:

1. **Window clock** — the vendor's usage pools (Claude 5h/weekly,
   Codex 5h/weekly, ZCode credits). Inputs: per-message token events +
   quota-feed events; plan config (tier, window sizes, multipliers)
   entered by the user per Account. Outputs: per-session/account/window
   aggregates, burn-down projections, reset timers, threshold alerts.
   This clock **triggers switching** (the suggestion at the wall), it
   is not the product's headline (D-01).
2. **Subscription clock** — renewal/expiry per Account. **User-entered
   and optional** (D-12): with a date, the indicator warns as it
   approaches; without one it shows `Unknown — no renewal date
   entered`, never inferred. Automated population only if AJ-E909's
   research proves a sanctioned, read-only, live-verified source for
   that Provider; unproven sources stay `unknown`.

Honesty rule unchanged: every number carries its confidence
(Measured/Computed/Estimated); per-model attribution where the provider
reports it. Storage: append-only `usage_events` + rollups.
## 7. Switching engine — unlimited, bidirectional, honest (ADR-0009/0011)

**Automation policy (unchanged product invariant).** The engine
*prepares*; the user *executes*. Every switch is bound to an explicit
user click; there is no code path that performs one without it
(`tests/guardrails.rs`).

The maintainer's sentence defines this section: switch "as many times I
want and switching back and forth as needed", with history, context and
rules working. Three switch kinds cover it:

| Kind | From → To | Mechanism range |
|---|---|---|
| **Switch-Account** | Instance → new Instance, same Provider, different Account | native-resume after transplant, or pack |
| **Switch-Provider** | Instance → new Instance, different Provider | pack replay (beta-badged fidelity) |
| **Switch-Instance** | Instance → new Instance, same Account | native resume |

### 7.1 Fidelity (per Provider-pair matrix; D-04/D-11)

Every (from-Provider, to-Provider) pair carries a per-artifact matrix —
**Copy / Translate / Regenerate / Drop** — over conversation turns,
rules/memory files, todos, diffs, and env. A pair's fidelity state is
`Beta — validated` only when its fixture matrix + live-binary dogfood
have passed (AJ-E210/E808); otherwise `Beta — unvalidated`, shown on
the switch itself. The word *lossless* is reserved for same-Provider
proven cases (native resume and validated transplants) and is **never**
used as a cross-Provider claim.

Superseded honest correction: the previous spec called T2
"effectively indistinguishable from having started on the target
account" — untrue as built (v0 transplants one newest file and drops
sibling subagent transcripts). Until a pair's matrix validates, its
ceiling is the matrix, not the label.

### 7.2 Switch execution (freeze → stage → verify → launch → rollback)

1. **Freeze** the source Instance's conversation state (quiesce point;
   the freeze protocol for mid-task switches is E2 scope — switching
   mid-flight when quota dies is the canonical workflow, so "start a
   new session elsewhere" is not an acceptable substitute).
2. **Stage** the portable context pack v2 (§3.3) — deterministic, with
   the redaction manifest and the *active* policy for the destination
   Provider visible before execution (D-14).
3. **Verify** — artifact manifest hash recorded on the SwitchEdge;
   target launch pre-checked (account signed in, project dir present).
4. **Launch** the target Instance (resume/transplant/replay per the
   matrix); record the edge in the Switch Graph (§3.1).
5. **Rollback/undo** — a compensating resume edge back to the source;
   the source Instance always remains resumable (transplants copy,
   never move or destroy).

### 7.3 The hybrid door (D-03)

- **One click, with an undo window** — when the pair is same-Provider
  and its fidelity is proven.
- **Wizard (brief, itemised)** — when the switch is cross-Provider,
  low-fidelity, or the graph detects divergence; it shows exactly what
  will Copy / Translate / Regenerate / Drop, and the beta badge. A
  lossy switch can never happen silently.

### 7.4 Switch-back and divergence (D-05)

Switch-back **resumes the original Account's native session** — the
epoch node points at the same `native_id`; work done on the interim
Account arrives as **one summarised delta turn** with the portable pack
attached. Copy-back over the original conversation is prohibited.
When *both* sides progressed (the normal case after real work),
divergence is surfaced via message-uuid three-way comparison and shown
to the user — never silently merged.

### 7.5 Project context sync (one click, diff-previewed)

Unchanged from before: rules files (AGENTS.md ↔ CLAUDE.md ↔
`.zcode/AGENTS.md`), Provider memory files, todos, uncommitted work — master
copy chosen by the user, diff before apply. Export/import of packs (any
session → any managed Provider) unchanged (T4).

### 7.6 Done checklist (§7 rewrite, AJ-E903)

- [x] One-way "handover" framing retired; three switch kinds defined
- [x] Switch Graph + freeze→stage→verify→launch→rollback + undo
- [x] D-03 hybrid door with enumerated wizard triggers
- [x] D-05 switch-back resume+delta; copy-back prohibited
- [x] Per-pair Copy/Translate/Regenerate/Drop matrix; beta badges;
      `lossless` restricted to proven same-Provider cases
- [x] D-14 pack redaction: user's remembered choice, visible pre-switch
- [x] Click-to-execute invariant restated; §3/§9/§12.1 cross-refs live

## 8. Security & privacy

- **Vendor credentials:** never read, copied, or stored by AI Juggler. They live wherever each harness keeps them inside the managed account dir (Claude: namespaced keychain; Codex: auth.json; ZCode: credentials.json). The app's keychain entries hold only AI Juggler's own config encryption key (if needed) — by default nothing.
- **Managed dirs** are 0700; the app runs with normal user privileges, no helpers, no agents.
- **Supply chain:** lockfile-audited deps, `cargo deny`, minimal JS in the webview (no remote content, CSP locked to self).
- **Updates:** Tauri updater, minisign-style signatures, verification cannot be disabled; OS notarization/signing from v0.1 (Apple Developer + Azure Trusted Signing, individual tier — budgeted).
- **Data:** everything local; "export diagnostics" redacts paths/tokens by default.

## 9. App & UX — Inspector-primary, indicators, Cleanup Centre (D-06/D-07)

### 9.1 Information architecture (ADR-0007/0008)

Left rail = the work surface: **Projects / Instances / Sessions** (three
tabs; the list lives in the rail; collapse to icons; ⌘B; New session at
its foot — recents + native folder picker + drag-drop, never typed
paths). Global oversight (Overview, Accounts, Usage, Fleet) sits
top-right. Selecting anything in the rail loads the **Right Inspector**
for that selection.

### 9.2 The Right Inspector — the daily surface (D-06)

Six tabs for **every** selection type (Session, Provider, Account,
Instance, Project):

| Tab | Content |
|---|---|
| Overview | identity, status, project, lineage (Switch Graph), quick actions |
| Usage | window-clock aggregates, projections, subscription clock (per D-12 states) |
| History | conversation/switch chronology from the Switch Graph |
| Quality/Performance | measurable signals only (D-07): error/retry rate, stuck/waiting evidence, context-window use, tokens & cost per completed task where known, process health — each labelled with its source; **no composite score** |
| Processes | the Instance's ledger-owned tree: CPU/RAM, children, leftovers, Cleanup Centre entry |
| Actions | switch…, switch-back…, stop, relaunch, export pack, focus window (desktop) |

Honest fields rule: a value appears only when measurable; otherwise
exactly `Unavailable — <reason>` (e.g. `Unavailable — Codex does not
expose context-window use`). Five states per surface: populated, empty,
loading, error, stale/offline.

### 9.3 Indicator grammar (the rail and board rows)

Every Instance row carries, with text+icon redundancy (never colour
alone — WCAG 1.4.1): Provider, Account, running model, state
(Starting/Running/WaitingUser/Stuck/QuotaBlocked/Finished/Stopped/
Failed/HandedOver), quota window state + reset countdown, cost-to-date
(where known), resources (CPU/RAM), child-process count, git
branch/dirty state. Fields the read model cannot supply yet render
`Unavailable — <reason>` (D-06). Density modes + legend + stale-data
rule ("older than N s, dimmed, show age") are E4 scope.

### 9.4 Cleanup Centre (D-08)

A dedicated surface: Leftover Inventory (ledger-verified orphans with
command line, cwd, held ports, age), the verify-before-kill sheet
(§5.2), per-Account tier controls (`Notify only` default /
`Clean safe (this Account)` opt-in), and the audit log of every kill
ever performed.

### 9.5 Positioning (D-09)

Tagline: **"Start anywhere. Switch when you hit the wall. Never lose
the thread — or leave a zombie behind."** Subtitle: "One console for
every Provider, Account, and Instance you juggle." ("zombie" is
glossed on first use in product copy.)

### 9.6 Done checklist (§9 rewrite, AJ-E904)

- [x] Inspector (6 tabs × 5 selection types) normative with honest-fields rule
- [x] Instances stay a first-class rail tab (merge-away reversed)
- [x] Indicator grammar table; text+icon redundancy; D-06 states
- [x] Cleanup Centre + tiers + audit specified
- [x] D-09 strings recorded; deep/11 supersessions noted (wireframes 01-09 own layout)

## 10. Self-update & Provider updates

- **App:** `tauri-plugin-updater` + GitHub Releases feed (`latest.json` via tauri-action), staged: download → verify signature → apply on quit or relaunch prompt; channel switch in settings; "check for updates" manual trigger; disable-able entirely.
- **Harness tracking:** adapters' `discover()` records installed versions; a background daily check against vendor release channels (GitHub/npm APIs) surfaces "Claude Code 2.1.301 available (you run 2.1.296)" with the adapter's verified-range status — the app never auto-updates a harness (its updater, your call).
- **Versioning & channels:** strict semver. Pre-1.0 ships as `0.x.0-beta.N` releases on the **beta** channel (early feedback); the **stable** channel opens with 1.0. Every release is a git tag `vX.Y.Z` with a CHANGELOG entry, updater-signed artifacts, and a notarized macOS build.
- **macOS signing/notarization:** Apple Developer Program membership (Vuxtra) provides Developer ID Application signing + `notarytool` notarization in CI (secrets via GitHub Environments). Open-source license unaffected — binaries are simply signed by the publishing identity, the standard model for company-published OSS.

## 11. Fleet: multi-machine operation — at launch, in beta (D-13, ADR-0013)

Fleet **ships with the launch** as a beta capability and is an
architecture constraint from day one: identity, ledger rows, ownership,
cleanup, and the Switch Graph are fleet-shaped (every record carries
`MachineId`; every remote action honors per-machine permissions), so
multi-machine never gets retrofitted. The security review (AJ-E710) is
a beta-exit gate. Scope at launch: pairing, remote Inspector (view),
remote switch and cleanup execution under explicit per-machine
permission. Detail contracts are owned by the E7 tasks; this section
states the normative frame.

**What Fleet never does, at any tier:** no credential sync, no login
automation, no traffic proxying, no cloud relay — the per-machine
invariants (§1, §8) hold on every node, and a node's spawn ledger is
never acted on by a console without that node's permission level
(`View` / `Control` / `NoAccess`).

AI Juggler instances link over the network so one console can see — and manage — every Instance on every machine you own. The design goal is **onboarding ease**: connecting a new machine is a pairing code, not a config file.

### 11.1 Topology & roles

- One binary, two roles: every instance is a **node** (serves its local state); any instance can act as a **console** (view/control the fleet). In practice one machine is "your" console; multiple consoles are allowed.
- **LAN-first, zero config:** discovery via mDNS/Bonjour (`mdns-sd`). Remote machines join through the user's own VPN (Tailscale/WireGuard) — AI Juggler never operates an internet relay or cloud service (privacy posture; also keeps the compliance story purely local).

### 11.2 Transport & security

- **gRPC over TLS 1.3 with mutual certificate auth (tonic + rustls)** — a standard, schema'd (protobuf), streaming-capable transport; protobuf contracts are versioned in-repo and breaking-change-linted in CI. (Codex's own app-server uses the same JSON-RPC-over-stdio pattern — the style is proven in this exact ecosystem.)
- Each instance generates an Ed25519 identity at first run; its self-signed certificate identifies the node.
- **Pairing:** the console displays a 6-digit code / QR; completing the exchange on the new node pins both identities. Unpaired connections are dropped — there is no anonymous mode.
- Commands from a console are authenticated to the node; per-machine permissions (e.g. "view-only") are set at pairing or later.
- **Link discipline:** the fleet link carries telemetry and control commands only — never model traffic, never vendor credentials, never raw transcripts except an explicit on-demand fetch for a user-executed cross-machine handover.

### 11.3 What syncs

- **Continuous (streamed):** heartbeats, machine resource stats, session-board deltas (status, tokens, quota impact), alerts.
- **On demand:** usage rollups, footprint stats, handover packs (fetched only when a cross-machine handover is executed).
- **Never leaves the machine:** vendor credentials (invariant), keychain contents; full transcripts stay local outside explicit handovers.

### 11.4 Fleet UX

- "Add a machine" wizard: enable Link on the node → code/QR → paired; machines appear as filters/columns across the board, dashboard, and usage views.
- **Cross-machine switch:** same hybrid door as local (§7.3); the fidelity matrix is computed across machines — transplants require the same Provider binary version and the repo checked out at the same path (native formats store absolute paths — surfaced honestly); pack-based switches are always available. Badged per D-11.
- Fleet alerts route to every console ("mac-mini-2: memory pressure").

### 11.5 Protocol versioning

- Protobuf contracts are versioned; mixed-version fleets supported within a minor window — nodes reject unknown command versions gracefully, and a console degrades to core telemetry when talking to an older node.

## 12. Storage layout

```
~/Library/Application Support/AI Juggler/        (macOS; XDG on Linux, %APPDATA% on Win)
├── juggler.sqlite          # accounts, launch_recipes, instances index, usage events, lineage, footprint stats (WAL)
├── accounts/<provider-id>/<account-id>/   # managed Provider state dirs (the isolation roots)
├── packs/                  # context packs + lineage metadata
├── fleet/                  # node identity keypair + paired-peer registry
├── logs/                   # rotated app logs (redaction-aware)
└── cache/                  # parser fixtures cache, rollups
```

Nothing is ever written inside `~/.claude`, `~/.codex`, or `~/.zcode` (the user's pre-existing single-account installs remain untouched; the smart-onboarding import (§12.1, ADR-0006) *reads* them and offers a managed copy).

### 12.1 Onboarding workflow (smart onboarding, ADR-0006; cross-Provider per pass2/03)

Onboarding assumes the user knows nothing about Provider internals, and
generalizes beyond Claude-only to every registered Provider:

1. **Detect** — the environment scan covers, per Provider: the vendor
   default dir, env-var overrides, home-naming patterns, **other
   manager tools' profile roots** (t3.codes, cc-switch, aliases),
   installed CLIs + versions, and known project directories. Every
   finding carries a provenance label — **Found** (verified on disk) /
   **Likely** (strong signal, e.g. env override target) / **Guessed**
   (name-pattern only) — shown inline, never silently.
2. **Recommend** — a set-level proposal ("import these 5 accounts, this
   launch setting each, ready to juggle") accepted with one click;
   imports are byte-copies, sources never touched (ADR-0006), with
   provenance and degraded-import notes where partial.
3. **Six flows, click-budgeted:** (a) first install; (b) new project;
   (c) add Provider; (d) add Account; (e) add Instance; (f) import from
   competitor/manual dirs. None requires typing a path.
4. **Login** — the Provider's own flow in an embedded terminal;
   sign-in queue with auto-detected state; isolation verified per
   account (state-dir containment check) before "ready".
5. **First-run policy choice** — pack redaction policy (D-14) and
   optional renewal dates (D-12) are asked once, explicitly.
6. **Summary** — what was created, what was skipped, originals
   untouched.

## 13. Testing strategy

- **Unit:** usage math (esp. ZCode credit formula vs. published examples), window reconstruction, pack rendering.
- **Fixture corpus:** committed, anonymized transcript samples per Provider × version; parsers tested against them; CI job re-verifies against live format dumps monthly (canary).
- **Fake Provider adapter:** a script-driven CLI implementing the adapter contract for e2e supervisor/stuck/handover tests — CI runs the full lifecycle headlessly on all three OSes.
- **Resource heuristics:** fixture process trees (the fake Provider spawning memory/CPU-hog children) drive overload alerts, admission-projection accuracy, and OOM-crash classification tests.
- **Fleet:** loopback multi-instance tests — two live instances in CI pairing over ephemeral ports, telemetry sync, permission enforcement, and a mixed-version downgrade matrix.
- **Guardrail tests:** static + runtime assertions that no code path writes outside managed dirs, performs network calls outside allowlist, exposes account-switching of live sessions, or executes any account switch / handover / project sync without a user-initiated command.
- **UI:** Playwright on the webview for board/wizard flows.

## 14. Milestones

Task-level breakdown, live tracking, and the per-milestone validation-gate process live in [docs/plan/EPICS.md](./plan/EPICS.md); the engineering standards and per-task Definition of Done live in [docs/plan/QUALITY.md](./plan/QUALITY.md). Every milestone closes with a full validation pass, a committed gate report (`docs/qa/gate-MX.md`), and user confirmation before the next one starts.

| M | Scope | Acceptance |
|---|---|---|
| **M0** | Workspace scaffold, CI (fmt/clippy/test, 3-OS build), adapter trait + fake Provider, domain model + SQLite | Fake session launches, is watched, and shows on a debug board |
| **M1** | Claude Code adapter: account wizard (login hand-off), isolated account dirs, parallel launch, instance board, embedded terminal *(pending decision #2)* | Two Claude accounts in parallel; zero writes to `~/.claude` |
| **M2** | Codex + ZCode adapters; adopt-existing flow; stuck detection v1; resource manager v1 (monitoring, overload alerts, admission advisor, crash recovery) | Three Providers, one board; stuck + overload heuristics pass fixture corpus; crashed session → one-click native resume |
| **M3** | Usage engine + dashboards + alerts (Claude Measured, ZCode Computed, Codex Estimated); tray | Quota rings within ±5% of vendor-reported for Claude/ZCode |
| **M4** | Handover: packs, takeover/return wizard, rules-file sync, export/import; lineage | Round-trip A→B→A preserves continuity (dogfood criterion) |
| **M5** | Hardening, signed + notarized self-update, Provider update tracking, onboarding polish; fleet transport spike (tonic/rustls prototype) → **v0.1 public beta** | Fresh-machine install → 2 parallel accounts < 5 min; updater works end-to-end on a signed build |
| **M6** | Fleet link: pairing wizard, mDNS discovery, telemetry sync, remote board (read-only) → **v0.2** | Second machine joins in < 60 s with a code; one board across two machines |
| **M7** | Fleet control: launch/stop/handover from a console, aggregated usage, fleet alerts; Windows/Linux bring-up begins → **v0.3** | Full manage workflow across 3 machines; cross-machine handover demo |
| **1.0** | Stabilization from beta feedback (protocol, adapters, UX) | Stable update channel opens |

## 15. Decisions

**Resolved (review rounds 1–3):**

1. **UI framework: React 18 + TypeScript + Tailwind + shadcn/ui.** ✅
2. **Embedded terminal ships in M1.** ✅
3. **License: MIT.** ✅
4. **Naming: binary `ai-juggler`, crates `juggler-*`.** ✅
5. **MVP order: Claude Code first (M1); Codex + ZCode in M2.** ✅
6. **Codex control plane: PTY-first (M2); app-server explored at M4.** ✅
7. **Platform: macOS-first (Universal, v0.x); Windows/Linux post-1.0; cross-platform core from day one.** ✅
8. **Fleet: gRPC over mTLS + mDNS pairing; one binary, console/node roles; LAN/user-VPN only — no cloud relay.** ✅ (M5 transport spike de-risks tonic/rustls before M6.)
9. **Release: strict semver; `0.x.0-beta.N` on the beta channel for feedback; stable channel opens at 1.0; macOS signing/notarization via Vuxtra's Apple Developer account.** ✅

**Open (non-blocking):**

1. Fleet wire format detail: protobuf-first vs JSON-RPC in the app-server style — decided at the M5 spike.
2. Whether the stable update channel opens earlier than 1.0 — revisit at v0.3 based on beta feedback.
