# Evidence manifest — AJ-E2016

Task: `docs/tasks/E20-engineering-and-security.md` AJ-E2016, adapter
building blocks and focused traits. Branch `release/0.4.1` (D-83). Status:
**DONE-EVIDENCED** (agent self-check, 2026-11-06). Both parts are
mechanical: no behaviour changed. One exception is named in R1: a URI
escaping that now also covers `#` and `?` for cc-switch's database path.

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | `crates/juggler-adapters/src/common/`, which names no tool. **`layout.rs`:** `Layout`, out of `claude_environment.rs`. **`home.rs`:** a config home's scan (`installs_from`), inspection, copy (links kept inside, never followed out), adoption, size and recency. It also holds the managed-folder marker (`MANAGED_MARKER`, `is_managed_dir`, `prepare_managed_dir_for`) and the private-folder and owner-only helpers, all out of `claude_environment.rs` and `claude_code.rs`. `claude_environment.rs` went from 992 lines to 239; Codex, Kimi Code, Grok Build and OpenCode now use `common::home` instead of reaching into Claude's module, and the layering allowlist lost `codex_environment -> claude_environment`. **`probe.rs`:** the `--version` probe that Grok Build, Kimi Code and OpenCode each carried as a copy. **`jsonl.rs`:** the bounded line reader (`BoundedLines`, `LINE_MAX`, `LinesRead`), re-exported from `transcript`. **`sqlite.rs`:** `uri_path` (two differing copies before) and `open_immutable`, now used by Vibe Kanban's, cc-switch's and OpenCode's readers. cc-switch's path is now escaped; a folder named with `#` or `?` opens the file it names. **ZCode's credit logic** (multipliers, peak window, credits, the `model_usage` read) moved to `zcode_usage.rs`, out of `quota_ingest.rs`, and is re-exported there so callers don't change. jscpd counts 315 fewer duplicated lines in the adapter crates (`scripts/debt_baseline.json` lowered). |
| R2 | PASS | `HarnessAdapter` (81 methods) is now the sum of eleven capability traits, each with at most 12 methods: `ToolIdentity` 4, `ToolDiscovery` 9, `ToolSignIn` 12, `ToolLaunch` 12, `ToolConfig` 7, `ToolInventory` 8, `ToolTranscripts` 8, `ToolQuota` 5, `ToolLive` 11, `ToolHandover` 2, `ToolRuntime` 3. A blanket impl makes every type with all eleven a `HarnessAdapter`, so `dyn HarnessAdapter`, `SharedAdapter` and the registry are unchanged. Default bodies stay where they were; only `id` is called across traits, which is why `ToolIdentity` is every other trait's supertrait. The ten impls were split per trait mechanically (`impl ToolX for Adapter`). `juggler_adapter::prelude` brings the contract in with one import. Doc links name the method's own trait. |

## Proof

- **Clippy** (`-D warnings`): clean for the workspace, with
  `--no-default-features`, and for `-p juggler-app --features lab-bridge`.
  Each run allowed one lint, `manual_is_variant_and`, at
  `juggler-app/src/features/tasks/launch.rs:90`: another session's commit
  e2cbebb2b (AJ-E1907), untouched here.
- **Tests:** see "Test run" below. The suites run were juggler-adapter,
  juggler-adapters (the conformance kit, `tests/adapter_conformance.rs`,
  included), juggler-core, juggler-app, juggler-handover,
  juggler-supervisor and juggler-agentbridge.
- **Static checks:** `check_layering.py` OK (one sibling import fewer),
  `size_budget.py` OK, `allow_reasons.py` (the moved exceptions now give
  reasons, two files off the list), `debt_baseline.py` (the adapter crates
  only shrank), `architecture_fresh.py` OK (ARCHITECTURE.md names the
  traits and `common/`), `ci_parity.py` OK.
- **`cargo-semver-checks`:** as ADR-0002 records, it joins CI with the
  first adapter crate published outside this workspace. Until then the
  trait changes with the workspace, in one change with every adapter, as
  here.

## Test run (2026-11-06, the final code)

- juggler-adapter, juggler-adapters (the conformance kit included) and
  juggler-core: every test passed.
- juggler-app, juggler-handover, juggler-supervisor and
  juggler-agentbridge (`--no-fail-fast`): 186 test binaries, 1,941
  passed, 0 failed, 38 ignored (hand-run live tests).
- One earlier run failed once, in
  `notifications::sources::tests::each_keep_alive_start_and_pause_is_a_notice_that_opens_the_switch`:
  another session's uncommitted edit to that file (AJ-E2112 R5), not
  touched here. It passed on the final run.
