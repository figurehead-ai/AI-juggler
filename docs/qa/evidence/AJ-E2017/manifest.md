# Evidence manifest — AJ-E2017

Task: `docs/tasks/E20-engineering-and-security.md` AJ-E2017, features as
modules: split `commands.rs` and `AppState`. Branch `release/0.4.1`.
Status: **DONE-EVIDENCED** (agent self-check, 2026-11-06). The split is
mechanical: no command's wire name, argument, result or behaviour changed.

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | **`commands.rs`: 15,831 lines and 95 commands before; 125 lines and none after.** The other 95 are in `features/<x>/`, each `commands.rs` (the thin `#[tauri::command]`s), `dto.rs` (the wire types) and the logic in `mod.rs` or a named file. `commands/costs.rs` (546 lines, 6 commands) became `features/cost` and `features/pricing`. 24 new feature folders, and new files in nine existing ones (table below). What stays in `commands.rs` is shared by every feature: `lock_storage_arc`, `now_millis`, `local_machine_id`, `ledger_ctx`, `core_to_app`, `find_account_arc`, `find_profile_with_account_arc` and `not_found_or_core`. It also keeps three re-exports the Floor's files still use; they go in the Floor's own pass. **Wire names unchanged:** the `app_commands!` registry, the dev bridge's routes and lab routes, and every user name the new paths. Those users are across `src`, `tests`, `examples` and `juggler-agentbridge/tests`; the commit (80f101da0) changes 320 files. `ts_types_fresh.sh` is OK: the 24 TS-exported types moved byte-identical. **Naming (`ipc_audit.py`):** a command in `features/<x>/` is named `x_…`. 17 whose wire name is older than their folder (`stop_session`, `instances_list`, `accounts_list`…) are in `ipc_audit.allow` under `name`, with the folder each lives in. **Per-feature managed state: not done here** (see "Left over"). |
| R2 | PASS | `crates/juggler-app/src/events.rs`: `AppEvent`, **29 events**, each with its wire string unchanged (the 11 in `ui/src/lib/bridge.ts`'s `EVENTS`, and the 18 the feature modules listen to). Every one of the **34 emit calls** outside the Floor goes through `EmitEvent::emit_event` / `emit_event_to`, so a misspelt event can't compile. The old `EVENT_*` consts stay for listeners, each defined from the enum (`AppEvent::X.name()`). So do the dev bridge's `FORWARDED_EVENTS` and `NOT_FORWARDED_EVENTS`. Tests: `every_event_has_its_own_wire_name`, and `nothing_emits_by_a_bare_name`, which fails on a `.emit(` outside `events.rs`. The Floor's three emits are exempt until its own pass. |
| R3 | PASS | [ADR-0030](../../../adr/0030-storage-concurrency.md), measured first with `crates/juggler-app/tests/storage_contention.rs` (ignored; `--ignored --nocapture`). The setup is 10 accounts, 50 working and 2,000 ended sessions, debug build. **Holds:** the projects list 56 ms p95, the read model 27 ms, the heuristics tick 20 ms (it reads every session), the working set 1.6 ms, a write 0.04 ms. **A page's wait for the lock:** 3 µs p95 alone; 2.6 µs p95 (45 ms max) with the ticks at 10× their rate. On its own read-only connection the read took 25.6 ms p95, no faster. **Decision:** one writer lock stays, and holds are short by rule (≤ 16 ms p95 in release). Per-repository connections stay the pattern for a repository with its own hot writer. A read pool comes only on a measured trigger. Nothing implemented beyond the measurement; the three long holds are the named follow-ups. |
| size_budget | PASS | `scripts/size_budget.py` counted a file only up to its first `#[cfg(test)] mod`. It now leaves out each test module, brace-matched (braces in strings, chars and comments ignored), and counts the rest. `--selftest` covers a module in the middle and two modules. `commands.rs` left `size_budget.allow`. Newly flagged and capped at today's size, as **extraction targets**: `crates/juggler-supervisor/src/supervisor.rs` 1,782 (spawn, stop and adoption) and `crates/juggler-floor/src/models.rs` 1,978 (one model per file). The largest new file is `features/switch/back.rs`, 1,342 lines outside its tests. |

## Where each section went

| `commands.rs` section | Now |
|---|---|
| App info, app settings, the accessibility pane | `features/app/{commands,dto}.rs` |
| AJ-E609 costs (`commands/costs.rs`) | `features/cost`, `features/pricing` |
| AJ-E611 ranking | `features/ranking` |
| AJ-E611 alerts tick | `features/alerts` |
| AJ-E611 reset queue | `features/reset_queue` |
| Quota display, usage source health | `features/quota/commands.rs` |
| Providers list (harnesses) | `features/harnesses` |
| Accounts: list, create, sign-in, web sign-in | `features/account/{mod,commands,dto}.rs` |
| AJ-E307 import, source inspect, use original | `features/account/import.rs` |
| Fee, cleanup opt-in, renewal, preview opt-in | `features/account/options.rs` |
| Environment scan; AJ-E302 ten-signal scan; AJ-E304 recommendation | `features/environment/{mod,scan,recommendation}.rs` |
| AJ-E309 funnel | `features/funnel` |
| AJ-E309 expert setup | `features/expert` |
| Launch settings' env overrides | `features/launch/recipe_env.rs` |
| AJ-E308 isolation check | `features/isolation` |
| Embedded terminal commands | `features/terminal/{pty,commands}.rs` |
| Profiles: list, create, delete | `features/profile` |
| Launching a session, the port warning's command | `features/launch` |
| AJ-E413 desktop launch and focus | `features/desktop` |
| AJ-E415 live preview | `features/preview` |
| Sessions list, stop, shared snapshot | `features/sessions/list.rs` |
| Lifecycle forwarding, attention heuristics | `features/sessions/attention.rs` |
| The read model, its emits and its tests | `features/sessions/{read_model,dto}.rs` |
| Dashboard counts | `features/dashboard` |
| Transcript scan, instance quality | `features/quality/transcript.rs` |
| Projects backfill; projects list | `features/projects/{backfill,list}.rs` |
| The juggle (session handover) | `features/handover/source.rs` |
| AJ-E205 preflight and door counts | `features/switch/preflight.rs` |
| AJ-E206 switch execute; replay; undo; graph edges | `features/switch/{execute,replay,undo,edges}.rs` |
| AJ-E207 switch-back | `features/switch/back.rs` |
| AJ-E209 targets; Switch Graph; resume | `features/switch/{targets,graph,resume}.rs` |
| AJ-E209 pack export | `features/packs/export.rs` |
| AJ-E208 redaction policy | `features/redaction` |
| AJ-E210 validation matrix | `features/validation` |
| AJ-E301 install gate | `features/providers` |
| AJ-E410 process rows; exit snapshot and tick | `features/processes/{instance,tick}.rs` |
| AJ-E412 running-model telemetry | `features/processes/running_model.rs` |
| AJ-E505 port map | `features/port` |
| AJ-E506 Cleanup Centre inventory | `features/cleanup/inventory.rs` |
| AJ-E507 verify-before-kill, the audit writer | `features/kill/{mod,audit}.rs` |
| AJ-E508 tiers sweep; AJ-E512 alerts; status | `features/cleanup/{sweep,alerts,status}.rs` |
| Recovery summary | `features/recovery` |
| One-version aliases (`legacy_aliases`) | each in its feature's `commands.rs` |

## Proof

- **Tests, before and after** (`cargo test -p juggler-app`; `cargo test -p juggler-agentbridge`):
  - juggler-app before: 1,662 passed, 0 failed, 37 ignored (156 test binaries).
  - juggler-app after: 1,667 passed, 0 failed, 38 ignored (158 binaries). The difference: +2 for `events.rs`'s tests; +1 ignored for the contention measurement; +3 passed and +1 binary for `replica_home_no_keychain.rs` and an `edition.rs` test, another session's commit (2e29d735f) that landed in between.
  - Every moved test runs: 14 `#[test]`s left `commands.rs`, and three of them moved to the code they test (`switch/execute.rs`, `switch/back.rs`, `app/dto.rs`).
  - juggler-agentbridge: 38 passed, 0 failed, before and after.
  - `tests/signal_paths.rs` now follows calls through `use` imports, so the split keeps its reach analysis whole. Its expected set of signalling commands is unchanged.
- **Static checks:** `cargo fmt --all --check` OK. `ipc_audit.py` OK (17 `name` allowlisted). `check_layering.py` OK. `webview_lint.py` OK (the accessibility pane's opener is allowlisted in `features/app/commands.rs` now). `size_budget.py --selftest` and `size_budget.py` OK. `allow_reasons.py`: every moved `#[allow]` gives its reason, and `commands.rs` (cap 36) left the list. Only the Floor's files still fail it, which is another session's work. `architecture_fresh.py` OK. `ts_types_fresh.sh` OK. `ci_parity.py` OK. `cargo doc -p juggler-app` with `-D warnings` OK: moved doc links name their full paths.
- **Clippy** (`-D warnings`): clean for this change in the workspace, with `--no-default-features` and with `--features lab-bridge`. Three findings remain, all from other sessions' commits already on the branch:
  - `clippy::panic` in `edition.rs`'s test helper (2e29d735f);
  - `doc_markdown` in `tests/replica_home_no_keychain.rs` (2e29d735f);
  - with `lab-bridge`, `dev_bridge::tests::every_read_command_is_served_or_says_why_not` fails on `places_offers` (f65f560b4, D-84), a Read command neither routed nor listed in `NOT_SERVED`.

## Left over

- **Per-feature managed state** (R1's "with per-feature managed state"): `AppState` still holds the moved features' state, such as `preview`, `tier2_pending` and the scan's cancel flag. Moving each into its feature's own state type is the next step. It touches `state.rs` and every test that builds an `AppState`, so it was kept out of this mechanical move.
- **ADR-0030's three long holds**, to fix in this order: the heuristics tick (working set only, file times after the lock), the projects list, and the read model (rows under the lock, the join after).
- **The Floor** (`features/floor/feed.rs`) still uses `crate::commands::{InstanceInfo, NeedsAttention, instances_shared_snapshot}`, and emits three events by name. Both move in the Floor's own pass; the re-exports are marked in `commands.rs`.
- **Extraction targets** the fixed size count found: `supervisor.rs` (1,782) and the Floor's `models.rs` (1,978). The largest moved file, `features/switch/back.rs` (1,342), is under budget but would split well into plan and execute.
