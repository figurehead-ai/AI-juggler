# Evidence manifest — AJ-E2019

Task: `docs/tasks/E20-engineering-and-security.md` AJ-E2019, typed command
wrappers. Branch `release/0.4.1` (D-83). Status: **DONE-EVIDENCED**
(agent self-check, 2026-11-06).

The task has one requirement paragraph; its parts are numbered here.

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | The ADR: `docs/adr/0029-typed-command-wrappers.md` chooses an in-repo generator (tauri-specta 2.0 is still a release candidate), with implementation notes on what the build found. |
| R2 | PASS | The generator and the wrappers, below. |

## R2: the generator

`crates/juggler-app/src/command_types.rs`, a test module (like ts-rs's
`export_bindings` tests), so `cargo test -p juggler-app --lib` writes
`ui/src/types/generated/commands.ts`:

- It reads the registry (`crate::COMMANDS`, the `app_commands!` table) and
  each command's function from its source with `syn` (by module path; a
  `pub use` re-export by its one definition), leaving out what Tauri
  injects (`State`, `AppHandle`, `Window`, `WebviewWindow`, `Webview`,
  `Request`, scopes).
- It writes a `Commands` map from each command's wire name (`rename`
  honoured) to `{ args; result }`: arguments camelCase (`rename_all =
  "snake_case"` honoured), an `Option` argument optional; `AppResult<T>` /
  `Result<T, _>` read as `T`; a struct or enum by its ts-rs name.
- **All 478 registered commands are typed** (129 take no arguments).
  `commands_ts_is_written` asserts the count equals the registry's.
- A type with no TypeScript form fails the test, naming the command, the
  parameter and the type. `UNTYPED`, the allowlist of types that may be
  `unknown`, is empty: nothing is left untyped.
- Self-tests: `types_map_as_the_adr_says` (every mapping rule, and what
  must fail), `a_command_reads_as_tauri_calls_it` (injected parameters,
  camelCase, `rename`, `rename_all`, a `Channel`, a `cfg(test)` module
  skipped, the rendered line), `an_unmapped_type_names_its_command`.
- 65 wire types had no ts-rs export and the UI typed them by hand; each
  now derives `TS` (53 in the app; 12 in `juggler-core` behind its new
  optional `ts` feature, which only the app turns on, exported by the
  generator's test). 64-bit integers are `number`, as elsewhere.

## R2: the wrappers

- `lib/ipc/transport.ts`: `call<K extends keyof Commands>(name, args)`
  returns `Promise<Commands[K]["result"]>`, arguments optional only when a
  command takes none it must have. `invoke` is no longer exported.
- 464 `invoke<T>("…", …)` calls in 86 `lib/ipc/*.ts` modules became
  `call("…", …)` by a script; `pick_path` in `transport.ts` too. The
  preview and test seams are unchanged (two test mocks now stand in for
  `call`); locals named `call` in 18 modules were renamed `called`.

### What the types found

`tsc` after the move, each a place where the UI and the backend disagreed:

| Command(s) | Mismatch | Done |
|---|---|---|
| `ranking_rows` | `admission` is a `String` in Rust; the UI reads `"ok" \| "tight" \| "no" \| null` | Rust field typed as that union (`#[ts(type)]`), the values checked in `ranking/mod.rs` |
| `alerts_tick` | `kind` a `String`; the UI reads five kinds | Typed as the union (values in `alerts/mod.rs`, `quota/alert_words.rs`) |
| `alerts_tick` | `quota` is left out when none (`skip_serializing_if`); ts-rs said `T \| null \| undefined`, the UI `T \| undefined` | `#[ts(optional)]` |
| `providers_discover` | `kind` a `String`; the UI reads `"terminal" \| "desktop"` | Typed as the union (`providers/mod.rs`) |
| `quota_display`, `quota_display_all` | `WindowDisplay.state` (7 states), `reset_basis`, `RenewalDisplay.state`, `ForecastDisplay.state` are `String`s; the UI reads unions | Typed as the unions (`window_state_name`, the renewal and forecast words) |
| `quota_display`, `quota_display_all` | `RunwayDisplay.series` is `Vec<(i64, f64)>`: ts-rs would say `bigint`, the wire carries numbers | `#[ts(type = "Array<[number, number]>")]` |
| `redaction_policy_get` | `source` a `String`; the UI reads `"global" \| "override" \| "unset"` | Typed as the union (`redaction/dto.rs`) |
| `switch_preflight` | `door` and each carry item's `how` are `String`s; the UI reads unions | Typed as the unions (`switch/preflight.rs`, `handover/switch_words.rs`) |
| `conversations_follow`, `subagents_follow` | The UI sent `channel: Channel \| null`; Rust requires the channel (null only toward the read-only bridge) | Without a stream the call goes through `withoutStream`, which the bridge refuses as before |
| The 96 commands answering `()` | Their wrappers say `Promise<void>` (65 `tsc` errors with a `null` result); `()` arrives as `null` | The map's result for `()` is `void` (ADR notes) |

No argument list disagreed: every call's names and types matched the Rust
signatures. Two type aliases kept only so the old IPC audit could read a
nested generic (`usageHistory.ts`) went.

Left over: the hand-written `…Data` interfaces for the newly exported
types (`RankedRowData`, `QuotaDisplayData`, `PreflightInfoData`, …) still
stand beside the generated ones; they now type-check against them and can
be folded in as their screens are touched.

## R2: the gates

- `scripts/ts_types_fresh.sh` covers `commands.ts` (it lives in
  `ui/src/types/generated`): a stale map fails it.
- `ui/.oxlintrc.json`: importing `invoke` from `transport` anywhere is an
  error (`no-restricted-imports` pattern), on top of `@tauri-apps/api/core`
  being refused outside `transport.ts`; `scripts/ui_lint.py`'s description
  says so.
- `scripts/ipc_audit.py` reads `call("…")` as a caller (its self-test
  updated).
- The dev bridge (the 0.4.1 `places_find` gap):
  `bridge_routes_match_the_commands` reads `dev_bridge/routes.rs` and
  checks each route's name and argument list against its command's
  signature without the `lab-bridge` feature; all routes match.
  `a_route_is_read_as_written` tests the reader.

## Runs (2026-11-06)

| Gate | Result |
|---|---|
| `cargo test -p juggler-app --lib` | 1043 passed, 6 ignored |
| `cargo clippy -p juggler-app -p juggler-core --all-targets --no-deps -- -D warnings`, and with `--features juggler-app/lab-bridge` | clean |
| `cargo fmt --all --check` | clean in these files |
| `bash scripts/ts_types_fresh.sh` | OK after commit |
| `python3 scripts/ipc_audit.py` (and `--selftest`) | OK |
| `python3 scripts/webview_lint.py` | OK |
| `pnpm exec tsc --noEmit -p .` | 0 errors |
| `pnpm exec vitest run --maxWorkers=4` | 281 files, 2139 tests passed |
| `pnpm lint` | OK |
| `pnpm build` | built |
| `python3 scripts/debt_baseline.py` | no change from `HEAD` (its failures there are other work's) |
