# Evidence manifest — AJ-E2011 (first part)

Task: AJ-E2011 "The cheap checks: stop the debt growing" (E20, in the
engineering-and-security epic, PR #101). Status: **IN-PROGRESS**. Four of
its nine checks are built here, on release/0.1, as the coordinator asked,
so the rest of Wave 1 is built under them. 2026-10-21, reviewer
claude-opus.

## What exists now

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

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 tool-literal lint | `tool_literal_lint.py --selftest` (4 cases) and the run on the tree | PASS |
| Req.2 layering | `check_layering.py --selftest` (5 cases: the allowed graph, a new sibling import, a gone listed import, an inward-pointing edge, a gone edge) and the run | PASS |
| Req.3 IPC audit | `ipc_audit.py --selftest` (4 cases, including a typo'd invoke) and the run | PASS |
| Req.6 size budget | `size_budget.py --selftest` (4 cases: caps and test exemptions, growth, an unlisted file, a file back within budget) and the run | PASS |

## Not built here (the rest of AJ-E2011)

Req.4 (the generated TS types' freshness), Req.5 (`clippy.toml` and the
workspace lints), Req.7 (cargo-machete, knip, jscpd), Req.8
(`[workspace.dependencies]`) and Req.9 (the QUALITY.md / ADR-0002 truth
pass) stay with E20.

## The one-time re-cap (2026-10-21, merge of main 3ccf2bd9)

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
