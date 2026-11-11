# Evidence manifest — AJ-E2011

Task: AJ-E2011 "The cheap checks: stop the debt growing" (E20). Status:
**DONE-EVIDENCED** (agent self-check, 2026-11-06, release/0.4.1). Four of
its nine checks came first, on release/0.1 (2026-10-21, below). The rest
were added on release/0.4.1 (D-83).

Each check has a self-test and runs in `just verify` and CI. Each starts
from an allowlist or baseline that only shrinks: a new violation fails,
and so does a listed one that went away without leaving the list.

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | Tool-id literals outside the adapters: `scripts/tool_literal_lint.py` + `tool_literals.allow` (selftest, 4 cases; `just tool-literals`, CI static). |
| R2 | PASS | Layering: `scripts/check_layering.py` against `docs/plan/layering.toml`, and no adapter importing a sibling (selftest, 5 cases). On 2026-11-06 it caught `juggler-link → juggler-core` once R7 removed that unused edge, and the layering file was updated. |
| R3 | PASS | `scripts/ipc_audit.py` + `ipc_audit.allow` (selftest, 4 cases). Covers: UI calls match registered commands (no allowlist), every command has a caller, async or a main-thread marker, bodies ≤ 40 lines, `<feature>_<verb>` names. |
| R4 | PASS | `scripts/ts_types_fresh.sh`: the TypeScript types the Rust tests generate must be committed as written (`just verify-static`, CI rust-build). |
| R5 | PASS | **Lint exceptions with a reason:** `scripts/allow_reasons.py` (selftest) fails any new `#[allow]`/`#[expect]` without `reason = "…"`. Today's 573 in 426 files are capped in `allow_reasons.allow`, which only shrinks. **Print and `dbg!`:** workspace lints `print_stdout`, `print_stderr`, `dbg_macro` = deny, with prints allowed in tests (`clippy.toml`). The lab-bridge build had 8 stray `eprintln!`s; they now go through the one allowed `early_note`. **`disallowed-methods`** (`clippy.toml`): `rusqlite::Connection::open` and `open_with_flags`. Every SQLite file now opens through `juggler_core::storage::open_hardened(_with)` (defensive mode, `trusted_schema` off). It found 12 connections that skipped AJ-E2006's hardening, among them all six readers of another tool's database (Codex goals, OpenCode, Vibe Kanban, cc-switch, ZCode usage) and five of AI Juggler's own stores. Test fixtures that build databases raw allow it, each with its reason. Clippy is clean in the default, no-default-features, dev-bridge and lab-bridge builds. |
| R6 | PASS | `scripts/size_budget.py` + `size_budget.allow`: Rust 1,500 non-test lines, TS 400 (selftest, 4 cases). |
| R7 | PASS | **cargo-machete** (`just supply-chain`, CI static; prebuilt, digest-checked) found two unused dependencies, now removed: `sha2` in juggler-handover and `juggler-core` in juggler-link. `tonic-prost`, used by generated code, is marked in its Cargo.toml. **knip** (`ui/knip.json`) and **jscpd** run through `scripts/debt_baseline.py` (selftest; CI ui job, preflight). Each file's unused exports and types, duplicate exports and duplicated lines are capped in `scripts/debt_baseline.json`, which only shrinks. knip also found the UI's `@tauri-apps/plugin-dialog` unused since AJ-E2003 moved the pickers to Rust, and the JS Tauri CLI unused: both removed. It found a visual test's broken type import, now fixed. jscpd runs as a per-file baseline: a new copy in any file fails, and code that only moves doesn't. |
| R8 | PASS | `[workspace.dependencies]` in the root `Cargo.toml`: the 29 dependencies two or more crates share, at one version each. 105 member entries now say `workspace = true` and keep their own features. `Cargo.lock` is byte-identical before and after, so no resolved version changed. |
| R9 | PASS | Truth pass (0584c8490). ADR-0002 now says `cargo-semver-checks` joins CI with the first adapter crate published outside the workspace. QUALITY.md says coverage is measured on every PR and reported, not blocking, with its targets. |

Performance budget (`just verify` at most +2 minutes): knip and jscpd
take about 3 s together, cargo-machete about 1 s, and the reason check
under 1 s.

## The first part (2026-10-21, release/0.1)

### What existed then

Each check has a self-test, runs in `just verify` (`just tool-literals`,
`just structure`) and in CI, and starts from an allowlist that only
shrinks. A new violation fails; so does a listed one that went away
without leaving the list.

| Check | Script | Rule (ENGINEERING-GUIDE) | Allowlisted today |
|---|---|---|---|
| Tool-id literals outside the adapters (Req.1, with AJ-E1525) | `scripts/tool_literal_lint.py` + `tool_literals.allow` | A2 | 150 sites in 15 files (none in the UI's screens) |
| Layering: the crate graph against `docs/plan/layering.toml`, and no adapter importing a sibling tool's module (Req.2) | `scripts/check_layering.py` | A1, A6 | graph: none (it already points inward); 3 sibling imports (Codex borrowing Claude's environment, transcript and adapter helpers) |
| IPC audit (Req.3): UI calls match registered commands (hard, no allowlist); every command has a caller; async or `// ipc: main-thread`; bodies ≤ 40 lines; `<feature>_<verb>` names (a command in `features/<x>/` starts with `x_`) | `scripts/ipc_audit.py` + `ipc_audit.allow` | A3, T1, I3, I4 | 14 uncalled, 192 sync, 2 long, 0 names |
| Size budget (Req.6): Rust 1,500 non-test lines, TS 400 | `scripts/size_budget.py` + `size_budget.allow` | C.10 | 27 files, each capped at today's size |

The IPC audit reads `#[tauri::command(rename = "…")]`, so the three
renamed commands (`instances_list`, `launch_recipes_list`,
`providers_list`) aren't flagged as missing. The commands added on
release/0.1 already pass every rule: `tools_catalog`, `session_prefs_*`
and `app_quit_plan` are async, and the quit sheet's pure commands carry
the marker.

### Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 tool-literal lint | `tool_literal_lint.py --selftest` (4 cases) and the run on the tree | PASS |
| Req.2 layering | `check_layering.py --selftest` (5 cases: the allowed graph, a new sibling import, a gone listed import, an inward-pointing edge, a gone edge) and the run | PASS |
| Req.3 IPC audit | `ipc_audit.py --selftest` (4 cases, including a typo'd invoke) and the run | PASS |
| Req.6 size budget | `size_budget.py --selftest` (4 cases: caps and test exemptions, growth, an unlisted file, a file back within budget) and the run | PASS |

### Not built then

Req.4 (the generated TS types' freshness), Req.5 (`clippy.toml` and the
workspace lints), Req.7 (cargo-machete, knip, jscpd), Req.8
(`[workspace.dependencies]`) and Req.9 (the QUALITY.md / ADR-0002 truth
pass) stay with E20.

### The one-time re-cap (2026-10-21, merge of main 3ccf2bd9)

Main's #108, #112 and #115 were written before these checks existed. At
their merge into release/0.1:

- #108's five commands became async through `ipc::blocking`, and
  `account_audit` was listed uncalled;
- Profiles' account sheets moved to `screens/AccountManage.tsx`
  (1,025 → 837 lines);
- the files the merge grew were re-capped where it left them. This was
  agreed with the coordinator, and each file is an extraction target, so
  the allowlist shrinks there.

| File | Cap before | Cap now | Extraction target |
|---|---|---|---|
| `crates/juggler-app/src/commands.rs` | 4,613 | 4,643 | AJ-E2017 (features as modules) |
| `crates/juggler-app/src/usage_stats.rs` | 2,084 | 2,085 | AJ-E2017 |
| `crates/juggler-supervisor/src/supervisor.rs` | 2,863 | 2,874 | AJ-E2017 |
| `ui/src/App.tsx` | 1,427 | 1,431 | AJ-E2017 scope, with AJ-E2018 (shell registry) |
| `ui/src/components/RightInspector.tsx` | 3,094 | 3,156 | AJ-E2017 scope, with AJ-E2018 |
| `ui/src/lib/bridge.ts` | 4,921 | 4,997 | AJ-E2017 scope, with AJ-E2018 (`lib/ipc/<feature>.ts`) |
| `ui/src/screens/FirstRun.tsx` | 1,519 | 1,527 | AJ-E2017 scope, with AJ-E2018 |
| `ui/src/screens/Home.tsx` | 816 | 818 | AJ-E2017 scope, with AJ-E2018 |
| `ui/src/screens/NewProjectSheet.tsx` | 413 | 425 | AJ-E2017 scope, with AJ-E2018 |
| `ui/src/screens/NewSession.tsx` | 520 | 527 | AJ-E2017 scope, with AJ-E2018 |
| `ui/src/screens/Profiles.tsx` | 736 | 837 | AJ-E2017 scope, with AJ-E2018 |

AJ-E2017 and AJ-E2018 are in `docs/tasks/E20-engineering-and-security.md`,
which comes with #101. When #101 is merged, this list goes into AJ-E2017's
handover note.
