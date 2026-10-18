# 05 — Process Trees, Zombies, Orphans and the Cleanup Centre (Pass 2)

> Second-pass review. First pass is cited, not restated: see
> `AI-Juggler-Maximum-Depth-Report-and-Proposal.md` §5 (supervisor FMEA
> F-1..F-10), §6 (scale model), §8 (target architecture / ADR-0010), and
> `docs/review/deep/08-architecture-data-supervisor-security.md` §5; UX
> cross-check `docs/review/full-competitive-ux-review.md` §Q5/F5.1.
> Canonical demand: `docs/expected-worfklow.md:15,17-18`. Spec anchor:
> `docs/SPEC.md:216-252` (§5.1–§5.3) and `docs/plan/EPICS.md:46,80`
> (M0-5, M2-6, M2-7). Code read in full:
> `crates/juggler-supervisor/src/supervisor.rs` (585 lines),
> `journal.rs` (216), `lifecycle.rs` (53), `error.rs` (39),
> `ring.rs` (129), `lib.rs` (24), plus callers in
> `crates/juggler-app/src/state.rs` and `commands.rs`.

## 0. Verdict in one paragraph

The maintainer names this as a headline problem, not hygiene:
tools “crash, exist, and their subprocesses, cli commands, terminals
and dev local services they started they end up running long after
the session closed, they end up being zombie processes” and the tool
“should have super easy way to track that, offer auomatic and manual
clean ups, all with verifications” (`docs/expected-worfklow.md:17-18`).
Pass 1 correctly found the kill-path defect (F-9) and the adoption
identity defect (F-2), but treated both as supervisor bugs to patch.
That under-weights the demand in three ways. First, the expected
workflow asks for a *product surface* — per-session resource and
sub-process visibility (“for each tool, session, we should [show] how
much resources it is using, cpu, ram, did it spawn any sub tasks, sub
processes, and allow to track those”, `docs/expected-worfklow.md:15`)
plus a cleanup experience — and no pass-1 section designs that surface.
Second, pass 1’s F-5 verdict (“our zombies: none”, Maximum-Depth §5)
answers the narrow Unix-reaping question and thereby closes the topic
the maintainer actually raised, which is mostly *orphaned live
processes*, not `Z`-state entries. Third, the two mechanisms that make
cleanup safe — process-group ownership at spawn time and a persistent
spawn ledger for descendants — do not exist in code today, so the
F-9 fix (“TERM tree → grace → KILL tree”) cannot be implemented
safely by editing `stop()` alone. This file designs the missing
system: ledger, identity, detection, port map, Cleanup Centre,
verify-before-kill, tiered auto-cleanup, audit, and crash recovery.

## 1. Terminology: maintainer language over code language

Maintainer vocabulary is explicit: “we should call them providers,
accounts, and instances of the tool” (`docs/expected-worfklow.md:11`),
with daily indicators “which provider it is, account, models currently
running” (`docs/expected-worfklow.md:13`). The code speaks a different
dialect. Mapping (adopt maintainer terms in UI; keep code names only
where renaming is churn):

| Maintainer term | Code term today (evidence) | Conflict / action |
|---|---|---|
| Provider | `HarnessId` “a harness kind, e.g. claude-code” (`crates/juggler-core/src/model.rs:44-47`); SPEC/PROPOSAL say “harness” (`docs/SPEC.md:12`, `docs/PROPOSAL.md:12`) | UI must say Provider; `HarnessId` stays as the internal id. Never show “harness” to users. |
| Account | `AccountId` / `Account` (`crates/juggler-core/src/model.rs:48-51`) | Aligned. Keep. |
| Instance (tool instance) | `SessionRecord` + `InstanceKind { Terminal, Desktop }` (`crates/juggler-core/src/model.rs:323-333,386-416`); ADR-0007 already decided “what the code called a launched session is, in product language, an instance” (`docs/adr/0007-instance-centric-information-architecture.md:28-33`) | Partially aligned in docs, not in types. Cleanup UI must group by Instance; the ledger key stays `SessionId` internally until a rename epic. Flag: `stop_session` command name (`crates/juggler-app/src/commands.rs:733-743`) exposes the old word across the Tauri bridge. |
| Session (conversation) | Also `SessionRecord`/`SessionId` — the same type carries both instance and conversation (`crates/juggler-core/src/model.rs:391-416`) | **Real conflict.** ADR-0007 separates instance (occurrence) from session (conversation) in prose but the struct merges them. Cleanup attributes processes to the *instance* (the spawner); handover attributes transcripts to the *session*. The ledger below therefore records both `session_id` (current key) and a future `instance_id`, so the split needs no re-journaling. |
| Profile | `Profile` = account + settings overlay (`crates/juggler-core/src/model.rs:170-184`) | No maintainer word. Keep as an Account-level launch preset; never present Profile as a peer of Provider/Account/Instance in cleanup UI. |
| (none) | Process / child / descendant (`crates/juggler-supervisor/src/supervisor.rs:341-356`) | New UI words needed: **Spawned process** (direct child), **Helper** (any descendant), **Orphan** (§2). Introduce in `juggler-core`, not as UI-only strings. |

Rule for the rest of this file: **Provider / Account / Instance**
are user-facing; `HarnessId / AccountId / SessionId` are cited only
as code evidence.

## 2. A precise taxonomy (pass 1 conflated three different things)

Pass 1’s F-5 (Maximum-Depth §5, `deep/08` §5) is correct that direct
children are waited (`crates/juggler-supervisor/src/supervisor.rs:157-171,242-259`)
and that `alive_pids` excludes `Zombie|Dead`
(`crates/juggler-supervisor/src/journal.rs:199-216`). It then
concludes “None; keep zombie filter”. That is the under-weighting:
the maintainer’s “zombie processes” (`docs/expected-worfklow.md:17`)
is the colloquial umbrella for at least five mechanically distinct
states, each needing different detection and different UX:

| # | State | Mechanism | Dangerous? | Correct verb |
|---|---|---|---|---|
| T1 | True zombie (`Z`) | Child exited, parent has not `wait()`ed; holds only a PID-table slot | No CPU/RAM; blocks PID reuse accounting | Reap (parent) or ignore if parent is `init`/launchd |
| T2 | Orphaned live descendant | Instance root exited/was killed; descendant reparented to PID 1, keeps running (dev server, MCP server, watcher) | **Yes** — RAM/CPU/ports/quota | Verify → terminate (this file’s core) |
| T3 | Detached-by-design survivor | Juggler itself exited; supervised roots survive intentionally (`crates/juggler-supervisor/src/supervisor.rs:47-51`) | No — this is SPEC §5.3 behaviour (`docs/SPEC.md:251`) | Re-adopt, never auto-kill |
| T4 | Leaked direct child of a live instance | Instance claims `Stopped`/`Finished` but its root still runs (stop failed silently) | Yes | Escalate ladder, surface as failed stop |
| T5 | Stranger on a recycled PID | PID reused after crash; journal still points at it (F-2) | **Critical if killed** | Never kill; identity check must fail closed |

Two consequences follow. (a) The UI must never label T2/T3 rows
“zombie”: users will learn the wrong mental model and fear T3
survivors that are working as designed. Suggested labels:
`Still running after its Instance closed` (T2), `Running — re-adopted`
(T3), `Unverifiable — will not be touched` (T5). (b) F-5’s “residual:
piped grandchildren reparented to init — out of scope, document it”
is precisely T2, which the expected workflow puts *in* scope
(`docs/expected-worfklow.md:17-18`). Pass 2 reverses that scoping:
T2 is the product; T1 is the footnote.

## 3. What pass 1 got right, and what it missed or under-weighted

Cited so this file extends rather than repeats (Maximum-Depth §5
table; identical text at `deep/08` §5):

- **Right — F-9 descendant leak.** Force-kill touches only the direct
  child (`crates/juggler-supervisor/src/supervisor.rs:149-151,234-236`);
  `descendants()` (`supervisor.rs:347-356`) is query-only and no caller
  in the kill path uses it — the only production `stop` callers pass a
  bare session id (`crates/juggler-app/src/commands.rs:740`,
  `crates/juggler-app/src/terminal.rs:200`). Agree, and §9 hardens it.
- **Right — F-2 PID reuse.** Adoption trusts PID liveness alone
  (`crates/juggler-supervisor/src/journal.rs:188-196`); `program` is
  stored (`journal.rs:35-40`) but never compared. Agree; §6.2 makes the
  check fail-closed and extends it from roots to every helper.
- **Right — F-3 adopted orphans are uncontrollable.**
  `alive_adopted_sessions` returns ids only
  (`crates/juggler-supervisor/src/supervisor.rs:432-438`) and no
  `ChildHandle` is rebuilt, so `stop` returns `UnknownSession`
  (`supervisor.rs:296-303`). Agree; §11.2 specifies the adopted handle.
- **Under-weighted — F-5 scoping.** See §2: T2 dismissed as residual.
- **Missed — no process-group is ever created.** A repo-wide search
  finds no `setpgid`/`setsid`/`process_group`/`killpg` anywhere under
  `crates/`. Piped spawns use bare `std::process::Command`
  (`crates/juggler-supervisor/src/supervisor.rs:129-140`); PTY spawns
  rely on the PTY session (`supervisor.rs:194-213`) without recording
  the PGID. Without a spawn-time PGID (or equivalent job object),
  “kill the tree” degrades to a racy ppid-walk snapshot — a helper
  can fork between the walk and the signal and escape. This is the
  load-bearing omission behind F-9.
- **Missed — descendants are invisible in the journal.** The journal
  records only root spawns/exits
  (`crates/juggler-supervisor/src/journal.rs:34-48`:
  `Spawned{session,pid,program,at}`, `Exited{session,pid,code,signal,at}`).
  Once the root exits, Juggler retains *no record* that a vite server
  or MCP helper ever belonged to it. Post-exit orphan detection is
  therefore impossible from current data, at any refresh rate.
- **Missed — no port dimension at all.** No listener/port enumeration
  exists in the supervisor or app crates (no `TcpListener`/LISTEN/`lsof`
  usage; `crates/juggler-supervisor/Cargo.toml:19` has `sysinfo` only).
  Yet the maintainer’s examples are port-holders: “dev local services
  they started” (`docs/expected-worfklow.md:17`). A cleanup list that
  cannot say “PID 8123 holds :5173” cannot earn user trust.
- **Missed — no resource attribution to show.** SPEC §5.2 already
  promises per-session tree RSS/CPU “attributed to the session even as
  children come and go (ppid-map tracking)” (`docs/SPEC.md:234`) and
  an RSS-anomaly alert (`docs/SPEC.md:243`); EPICS carries it as M2-6
  (`docs/plan/EPICS.md:80`). Code has neither: `descendants_of`
  (`crates/juggler-supervisor/src/supervisor.rs:557-579`) returns bare
  `Vec<u32>` with no RSS/CPU, and each call builds a fresh
  `System::new()` full-process refresh — the cost pass 1 flagged at
  Maximum-Depth §6.2(3). The gap is a data model, not a widget.
- **Missed — `children` map grows forever.** Insert-only
  (`crates/juggler-supervisor/src/supervisor.rs:285-287`); `stop`
  never removes, exit never evicts (F-6). A Cleanup Centre that scans
  this map inherits the leak as phantom “Running” rows.
- **Wrong-by-omission — SPEC’s own promise is untestable today.**
  SPEC §5.1 promises “zombie/orphan detection on app restart”
  (`docs/SPEC.md:224`) and §5.3 re-adoption “by PID + log
  correlation” (`docs/SPEC.md:251`). The only restart test spawns a
  lone `sleep 300` with no children
  (`crates/juggler-supervisor/tests/supervisor.rs:118-151`), and the
  only tree test asserts descendants exist but never that they die
  (`tests/supervisor.rs:153-173` — it stops the root and waits only
  for the root’s exit event). The fixtures in SPEC §10 (“fake harness
  spawning memory/CPU-hog children”, `docs/SPEC.md:439`) do not exist.

## 4. Design principle: ownership must be created at spawn, not inferred later

Every safe cleanup decision reduces to one question — *did an Instance
we manage create this process?* Today that is inferred after the fact
from a ppid walk (`crates/juggler-supervisor/src/supervisor.rs:557-579`)
that is correct only for one instant and only while ancestors live.
Three spawn-time mechanisms replace inference with ownership:

1. **Process group per Instance (Unix).** Each root is spawned as a
   group leader (`setsid` for piped children via `Command::pre_exec`;
   PTY children already get a controlling terminal — record the PGID
   explicitly instead of assuming it). Helpers that do not themselves
   call `setsid` stay in the group, so `killpg` reaches the dev server
   that double-forked away from the ppid chain. This is the standard
   Unix ownership primitive; Juggler currently creates none (§3).
2. **Job Object per Instance (Windows).** `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`
   gives the same guarantee with OS enforcement. Deferred to the
   Windows bring-up (EPICS M7-5, `docs/plan/EPICS.md:80` rows) but the
   trait in §12 must leave the seam now.
3. **Spawn ledger (all platforms).** A persistent record of every
   root *and every observed helper*, written before it is needed.
   The ledger is what survives the root, the app, and the reboot —
   the three moments ppid inference fails.

macOS note: there is no Linux-style cgroup to lean on; PGID + ledger
+ endpoint verification is the portable trio. Where a helper
deliberately daemonises (`setsid` + double-fork, e.g. some dev
servers), PGID escapes too — the ledger’s ppid snapshots (§5) plus
port/cwd corroboration (§7) are the fallback, and the UI must say
`escaped its process group` honestly rather than claim containment.

## 5. The spawn ledger (new persistent state)

The current journal (`crates/juggler-supervisor/src/journal.rs:34-48`)
is an append-only root log. It stays as the crash-recovery log, but
cleanup needs a queryable ledger. Proposal: a `process_ledger` table
in the existing SQLite store (WAL, per SPEC §3 storage), written by
the supervisor through a narrow trait so `juggler-supervisor` stays
storage-agnostic. Columns (Rust type in §12):

- Identity: `pid`, `pgid`, `started_at` (process start time from the
  OS, not wall-clock spawn time — the F-2 fix), `exe_path`,
  `cmdline_hash` + truncated `cmdline` (display), `boot_id` (changes
  every reboot; kills stale-PID false positives outright).
- Attribution: `session_id` (Instance), `provider` (`HarnessId`),
  `account_id`, `project_id`, `root_pid`, `parent_pid` at first
  observation, `depth` (1 = direct child), `first_seen`, `last_seen`.
- Role guess: `role` ∈ `Harness | Subagent | DevServer | McpServer |
  TerminalShell | Watcher | BuildTool | Unknown` — inferred from exe/
  cmdline/port heuristics, always shown as a guess, never as fact.
- Lifecycle: `state` ∈ `Running | Exited | Orphaned | Adopted |
  Unverifiable`, `exit_kind`, `reaped_by_us`.
- Evidence: `listens_on` (ports), `cwd`, peak RSS/CPU (§8).

Write discipline matters more than schema. Three rules fix F-7’s
ordering bug (spawn precedes journal at
`crates/juggler-supervisor/src/supervisor.rs:107-114`, so a journal
failure orphans an unrecorded process): (1) write an `Intent` row
(state `Starting`, no PID yet) *before* spawn; (2) patch in PID/PGID/
start-time immediately after; (3) never fail the spawn because the
ledger failed — degrade to `ledger_degraded` health flag instead.
Helper rows are upserted by the reconciler (§6) at most once per
2 s tick per Instance, batched in one transaction.

Retention: exited-helper rows are kept 7 days (audit + “what did I
kill?” history), then pruned by the same job that prunes journal
entries. Rows for processes still running are never pruned.

## 6. Liveness and orphan detection

### 6.1 The reconciler loop

One shared `sysinfo::System` refreshed on the SPEC §5.2 cadence
(“~2 s for supervised process trees”, `docs/SPEC.md:231`) replaces
today’s per-call `System::new()` in both
`crates/juggler-supervisor/src/supervisor.rs:557-579` and
`crates/juggler-supervisor/src/journal.rs:200-216` (the cost flagged
at Maximum-Depth §6.2(3)). Each tick the reconciler:

1. Refreshes once; builds the ppid map in-memory (no allocation of a
   fresh map per query as `descendants_of` does today).
2. For each live Instance root: walks descendants, upserts helper
   rows, refreshes RSS/CPU/ports, detects role changes.
3. For each root that exited since last tick: marks helpers whose
   ppid is now 1 (or whose ancestor chain no longer reaches a live
   Juggler-owned root) as `Orphaned` (T2) — *after* a grace window
   (§6.3), because well-behaved helpers exit milliseconds after their
   parent and must not flicker into the Cleanup Centre.
4. For each ledger row in `Running|Orphaned|Adopted`: re-verifies
   identity (§6.2). A failed check flips the row to `Unverifiable`
   (T5) and permanently removes it from every kill path.
5. Emits deltas as lifecycle events; the UI never polls `ps` itself.

### 6.2 Identity verification (fail closed)

A PID is a name, not an identity. Before *any* signal, and before a
row is even shown as killable, three facts must agree with the
ledger: (a) OS process start-time equals the recorded start-time;
(b) resolved exe path equals the recorded exe (symlinks resolved);
(c) `boot_id` matches the current boot. This extends pass 1’s F-2
fix (Maximum-Depth §5: “verify /proc/ps exe path + start-time”) from
adoption-time roots to every helper at kill-time, because helpers are
exactly where PID reuse bites during a long session. Cost: one
`stat`/`proc_pidinfo` per candidate per tick — negligible beside the
2 s full refresh SPEC already budgets (`docs/SPEC.md:231`).
External background on why start-time matters: PID reuse is a
documented hazard of PID-only tracking —
[man7.org proc(5) on PID reuse](https://man7.org/linux/man-pages/man5/proc.5.html).

### 6.3 Orphan grace and flap control

Defaults (all user-tunable, stored per Provider): `orphan_grace =
30 s` after root exit before a helper is listed; `hog_window = 60 s`
sustained breach before a hog alert (§8); `adopted_idle = 24 h`
before a T3 survivor is *suggested* (never auto-cleaned, §10). A
helper that exits during grace is journaled as a normal exit and
never surfaces. Two consecutive ticks must agree a process is
orphaned — single-tick sightings are hidden, which also absorbs
ppid-map races during fork storms (e.g. `cargo build` spawning
dozens of short-lived `rustc` helpers).

## 7. Port occupancy map

Ports are the evidence users already understand: “something is on
:3000”. The map joins three sources per reconciler tick: the ledger
(which PID belongs to which Instance), the OS listener table, and a
curated well-known-dev-port hint list (3000, 4173, 5173, 8000, 8080,
9229 — hints only, never identity). Implementation: `sysinfo`
0.39 (`crates/juggler-supervisor/Cargo.toml:19`) does not expose
listeners portably, so use `netstat2` on macOS/Linux or
`GetExtendedTcpTable` on Windows behind a `PortProbe` trait (§12);
fall back to “ports unknown on this platform” rather than shelling
out to `lsof` (slow, permission-flaky, and unparsable under load).

Each port row shows: port, owning PID → Instance → Provider/Account/
Project, listener age, and a collision badge when the owner is *not*
the Instance the user is about to launch into that project. Two
user-facing wins follow directly: launch can warn “:5173 is already
held by an orphan from Tuesday’s Instance” *before* vite fails with
`EADDRINUSE`, and the Cleanup Centre can offer “free this port” as a
goal-oriented action that resolves to a verified PID kill (§9).
Processes owned by the user outside Juggler (their own `npm run dev`)
appear in the map as `Not managed by Juggler` and are display-only —
seeing them prevents the classic mis-kill, and §13-F3 requires it.

## 8. Resource-hog alerts (attribution before alarm)

SPEC §5.2 promises tree RSS/CPU per session (`docs/SPEC.md:234`),
anomaly alerts at “sudden ≥ 3× growth” (`docs/SPEC.md:243`), and the
historical `footprint_stats` advisor (`docs/SPEC.md:235-237`); none
exists in code (§3). Pass 2 scopes the *minimum* that makes cleanup
decisions informed, deferring the full advisor to M2-6
(`docs/plan/EPICS.md:80`):

- Per-helper and per-Instance rollups each tick: RSS, CPU% (delta over
  the 2 s window, respecting `sysinfo`’s two-sample CPU requirement),
  cumulative CPU time. Stored as peaks in the ledger (§5).
- Hog rules (sustained over `hog_window`, §6.3): Instance tree RSS
  above the per-Provider learned p95, or any single helper above an
  absolute floor (default 2 GiB), or CPU above 90% of one core for
  the full window. A hog alert names the top helper, not just the
  Instance — “`node (vite)` under Instance X holds 3.1 GiB” is
  actionable; “Instance X is heavy” is not.
- Alerts are passive suggestions with a staged action, per the
  automation invariant: the app “prepares; the user executes”
  (`docs/SPEC.md:26`). A hog alert may pre-fill a Cleanup Centre
  selection; it may never signal a process (§10).
- Idle-orphan nudge: a T2 orphan idle (CPU ≈ 0, no listened-port
  traffic signal available portably — so CPU+age only) for 1 h raises
  one low-severity suggestion. No repeated nagging: once per orphan.

## 9. The Cleanup Centre (product surface)

A new global surface (top-right global menus per ADR-0007’s
“global / setup mode”, `docs/adr/0007-…md:20-23`; not a rail peer of
Projects/Instances/Sessions). It is the answer to “super easy way to
track that” (`docs/expected-worfklow.md:18`) and to the per-Instance
resource demand (`docs/expected-worfklow.md:15`).

### 9.1 List model

One row per *process* (not per Instance), grouped by Instance with a
collapsed “healthy tree” default. Columns: state badge (T1–T5 labels
from §2), role guess, exe/cmdline (truncated, expandable), PID/PGID,
age, RSS/CPU (live), ports, Provider · Account · Project, and the
verification badge (§9.2). Filters: `Needs attention` (default:
Orphaned + hogs + port collisions), `All managed`, `Re-adopted`,
`History (7 days)`. Every row carries “Why does Juggler think this is
mine?” expanding to the evidence chain (spawned-by root, first-seen
ppid, PGID match, exe/start-time check). Empty state is a success
state: “Nothing left behind — N Instances running clean.”

The Instance panel (ADR-0007 Decision 2: “status, account/profile,
kind, age, resources, actions”, `docs/adr/0007-…md:35-40`) embeds the
same rows filtered to that Instance, satisfying
`docs/expected-worfklow.md:14-15` without a second data path.

### 9.2 Verify-before-kill flow (no exceptions)

Killing is a three-step staged action; there is no one-click kill
anywhere, including the Instance panel and hog alerts:

1. **Verify (automatic, on selection).** Re-run §6.2 identity checks
   *now* (not from the last tick), re-read exe/cmdline/cwd/ports, and
   show a confirmation sheet: full cmdline, cwd, ports that will be
   freed, parent Instance and its state, and what else shares the
   PGID. If verification fails, the sheet becomes an explanation
   (“This PID now belongs to a different program — Juggler will not
   touch it”) with the kill button removed, not disabled-with-tooltip.
2. **Choose scope.** `This process only` / `This process and its
   helpers` (PGID ladder) / `Whole Instance tree`. Default is the
   narrowest that covers the user’s stated goal (“free :5173”
   defaults to process-only).
3. **Confirm with consequence text.** The button names the outcome:
   `Terminate vite (PID 8123) and free :5173`. Destructive confirms
   follow the house pattern; there is no “Don’t ask again” for kills.

### 9.3 Manual TERM → KILL ladder UX

Extends the existing root-only ladder
(`crates/juggler-supervisor/src/supervisor.rs:290-318`: SIGTERM, poll
10 ms, force-kill) to the verified scope: TERM the PGID (or the single
PID for process-only), show a live countdown with per-process exit
ticks as each helper dies, then KILL survivors only after the grace
(default 5 s, matching today’s `stop_session` grace at
`crates/juggler-app/src/commands.rs:740`). The user can cancel during
grace; a helper that ignores TERM is labelled `Ignored SIGTERM` before
any KILL is offered — surprise SIGKILLs are how data is lost (a dev
server is cheap to kill; a database helper is not). On Windows the
ladder is `TerminateProcess` only, and the UI must say so.

After the ladder, the sheet reports per-process outcomes (exited on
TERM / needed KILL / already gone / refused) and writes the audit
record (§11). “Already gone” is a success, not an error.

## 10. Safe auto-cleanup policy (opt-in tiers)

The expected workflow asks for “auomatic and manual clean ups”
(`docs/expected-worfklow.md:18`), but SPEC’s automation invariant
forbids state-changing actions without a click, with one narrow
exception (opt-in per-profile auto-restart, `docs/SPEC.md:26`).
Auto-cleanup therefore needs the same treatment: an explicit,
narrowly-scoped, off-by-default opt-in, recorded as a SPEC amendment
alongside the auto-restart exception — not a quiet settings toggle.
Pass 1 never surfaced this conflict; resolving it is a precondition,
not a detail. Proposed tiers:

| Tier | Default | What it may do without a click | Hard exclusions |
|---|---|---|---|
| 0 Observe | **On** | Detect, list, alert, pre-stage cleanup sheets | Never signals anything |
| 1 Tidy verified orphans | Off (opt-in per Provider) | After `orphan_grace` + identity re-verified at kill time: TERM→KILL helpers whose Instance root exited *cleanly* (code 0) and which hold no unsaved-work role (`DevServer`, `Watcher` only) | Never roots; never `McpServer`/shell/DB roles; never after a crash exit |
| 2 Tidy after crash | Off (separate opt-in) | As Tier 1, but for crash-exited Instances, after 5 min and one user-visible notification with Undo window | Never T3 adopted survivors; never `Unverifiable`; never within `adopted_idle` |
| 3 Hog shed | **Never automatic** | — (alert + staged sheet only, §8) | A busy process is not a dead process; no tier may kill for resource use alone |

Invariants for all tiers: identity is re-verified in the same tick
as the signal (§6.2); every automatic action writes the §11 audit
record *before* signalling; a global “Pause automatic cleanup”
kill-switch lives in Settings and in the Cleanup Centre header;
any verification failure silently demotes the candidate to Tier 0
(list-only). Tier 1/2 are suspended for 24 h after any Juggler crash
recovery (§11.2) until the user re-confirms — the moment identity
data is least trustworthy is right after a crash.

## 11. Audit log and recovery after Juggler itself crashes

### 11.1 Audit log

Append-only `cleanup_audit` (SQLite table + JSONL export): timestamp,
actor (`user:<command>` | `auto:tier-1` | `auto:tier-2`), candidate
PID/PGID/exe/cmdline-hash, Instance/Provider/Account/Project,
identity-check results, scope chosen, signals sent with per-PID
outcomes, ports before/after. Written *before* the first signal
(intent) and completed after (outcome) — a crash mid-ladder still
leaves a truthful “we TERM’d PGID 8120, outcome unknown” record.
The Cleanup Centre History filter (§9.1) reads this log; Settings
offers export. No audit record, no kill: if the audit write fails,
manual kills warn-and-require-retry and automatic tiers stand down.

### 11.2 Juggler crash recovery

Today: children survive drop by design
(`crates/juggler-supervisor/src/supervisor.rs:47-51`), the journal
replays roots (`crates/juggler-supervisor/src/journal.rs:151-196`),
and `AppState` reconciles non-adopted `Starting|Running` sessions to
`Stopped` (`crates/juggler-app/src/state.rs:53-72`). Gaps pass 1
named (F-1/F-3) plus two it did not: helpers are absent from the
journal so a crashed Juggler forgets every dev server it ever saw,
and adopted roots get no `ChildHandle` so they cannot be stopped.

Recovery sequence with the ledger: (1) load ledger rows in
`Running|Orphaned|Adopted`; (2) identity-verify each (§6.2) —
failures become `Unverifiable` (listed, never killed); (3) rebuild
adopted handles (output unavailable, stop = verified PGID ladder) —
closing F-3; (4) helpers of roots that died *while Juggler was down*
enter `Orphaned` with their original first-seen attribution, so the
user sees “left behind while Juggler was closed”, distinct from
live-session orphans; (5) show a one-time recovery summary (“3
Instances re-adopted, 2 helpers left behind, 1 process could not be
verified and was left alone”) — silence here is how trust dies.
Auto-cleanup tiers stay suspended (§10) until dismissed.

## 12. Supervisor architecture changes (Rust-level)

New module `crates/juggler-supervisor/src/tree.rs` (ledger types,
reconciler) and `ports.rs` (`PortProbe`); `lib.rs` exports grow from
today’s five modules (`crates/juggler-supervisor/src/lib.rs:14-24`).
Core types (signatures are the contract; bodies follow §5–§11):

```rust
pub struct ProcessIdentity { pub pid: u32, pub pgid: u32,
    pub start_time: i64, pub exe: PathBuf, pub boot_id: String }
pub enum HelperRole { Harness, Subagent, DevServer, McpServer,
    TerminalShell, Watcher, BuildTool, Unknown }
pub enum OrphanState { Running, Exited, Orphaned, Adopted, Unverifiable }
pub struct LedgerRow { pub identity: ProcessIdentity,
    pub session: SessionId, pub root_pid: u32, pub parent_pid: u32,
    pub role: HelperRole, pub state: OrphanState,
    pub ports: Vec<u16>, pub peak_rss: u64 }
pub trait LedgerStore: Send + Sync {
    fn intent(&self, s: &SessionId) -> Result<LedgerId, SupervisorError>;
    fn spawned(&self, id: LedgerId, p: &ProcessIdentity) -> Result<(), SupervisorError>;
    fn upsert_helpers(&self, rows: &[LedgerRow]) -> Result<(), SupervisorError>;
    fn candidates(&self) -> Result<Vec<LedgerRow>, SupervisorError>;
}
pub fn verify(pid: u32, expect: &ProcessIdentity) -> Verification; // §6.2, fail-closed
pub fn terminate_group(pgid: u32, grace: Duration) -> LadderOutcome; // TERM→poll→KILL
```

Spawn changes: piped path gains `pre_exec(setsid)` + PGID capture
(against today’s bare `Command` at
`crates/juggler-supervisor/src/supervisor.rs:129-140`); PTY path
records `child.process_id()`’s PGID explicitly
(`supervisor.rs:214-219`). `Supervisor::stop` keeps its signature for
the two existing callers but delegates to `terminate_group` over the
verified tree — F-9 fixed at the primitive, not by bolting a ppid
walk onto `stop`. `descendants()` is kept for compatibility but
reimplemented on the reconciler’s cached map. `ChildHandle` gains
`identity: ProcessIdentity` and adopted-handle construction.
Errors: add `SupervisorError::Unverifiable { pid }` and
`::LedgerDegraded` to the closed enum at
`crates/juggler-supervisor/src/error.rs:10-39` — callers must handle
“refused to kill” as a typed outcome, never as `UnknownSession`.
No new `unsafe` beyond the existing `nix` signal calls
(`crates/juggler-supervisor/Cargo.toml` unix `nix` dep); `pre_exec`
closure must be async-signal-safe (single `setsid` call only).

## 13. Safety FMEA (cleanup-specific; extends Maximum-Depth §5)

| # | Failure | Harm | Mitigation in this design |
|---|---|---|---|
| C1 | PID reused by user’s own process, then killed | Data loss in an unrelated app — the worst outcome this feature can produce | §6.2 triple check at kill-time; T5 rows have no kill affordance (§9.2) |
| C2 | Helper `setsid`-escapes PGID; group kill misses it, UI claims success | False confidence; port stays held | Post-ladder port re-probe (§9.3); escaped helpers labelled, killed PID-only after re-verify |
| C3 | Shared PGID with user’s terminal shell | Killing the group kills the user’s shell | Spawn-time dedicated PGID per Instance (§4); confirm sheet lists every PGID member (§9.2) |
| C4 | MCP server holds unsaved state; Tier-1 kills it as “watcher” | Work loss | Role exclusions (§10); role is a guess, so only `DevServer|Watcher` are ever auto-eligible |
| C5 | Adopted survivor (T3) mistaken for orphan after Juggler crash | Kills work the user deliberately kept (SPEC §5.3, `docs/SPEC.md:251`) | `Adopted` is a separate state; Tiers 1–2 exclude it; recovery summary (§11.2) |
| C6 | Fork storm floods ledger/UI | Perf death, alarm fatigue | 2-tick confirmation, batched upserts, collapsed healthy trees (§6.3, §9.1) |
| C7 | Audit/ledger write fails, kill proceeds unrecorded | Unaccountable destruction | §11.1: no record, no kill (auto) / warn-and-retry (manual) |
| C8 | Clock/boot change makes start-time checks flaky | Everything flips `Unverifiable`, feature looks broken | `boot_id` gate first; mass-unverifiable triggers a health banner, not row spam |

## 14. Acceptance criteria (testable, fixture-first)

Requires the SPEC §10 fixture harness (`docs/SPEC.md:439`) extended
to spawn a port-holding, TERM-ignoring, and `setsid`-escaping helper:

1. Fake Instance spawns vite-like helper on :5173 → ledger shows
   helper + port within 2 ticks; root exits cleanly → helper lists
   as Orphaned only after grace, with correct Provider/Account/Project.
2. `stop` on a live tree kills root *and* helpers (extends
   `crates/juggler-supervisor/tests/supervisor.rs:153-173`, which
   today never asserts helper death); :5173 verifiably free after.
3. PID-reuse drill: kill helper, respawn a stranger reusing the PID
   (or simulated identity mismatch) → row becomes `Unverifiable`,
   no kill control rendered, automatic tiers skip it.
4. Juggler-crash drill: drop supervisor with live tree (pattern of
   `tests/supervisor.rs:118-151`), restart → roots re-adopted *with*
   stop control, down-period helpers listed as left-behind, recovery
   summary shown once.
5. Tier-1 drill (opt-in): clean-exit orphan dev-server auto-TERMed
   after grace with pre-written audit record; MCP-role helper in the
   same tree untouched. Default install (Tier 0) signals nothing —
   asserted by a guardrail-style test alongside the invariant at
   `docs/SPEC.md:26`.
6. Budget: reconciler at 50 Instances keeps supervisor CPU < 1%
   (SPEC budget, `docs/SPEC.md:24`) and adds no per-query
   `System::new()` (regression test counts refreshes per tick).

Sequencing: ledger + PGID + reconciler first (unblocks F-2/F-3/F-9), Cleanup Centre read-only second, manual ladder third, Tier 1/2 last and only after the SPEC automation amendment. That is M2-6/M2-7 scope (`docs/plan/EPICS.md:80`) plus one new task, **M2-9 Cleanup Centre**, gated at V2.


