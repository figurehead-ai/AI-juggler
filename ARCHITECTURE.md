# Architecture

Revisited for: 0.4 (2026-11-06). Read at each milestone gate and brought
up to date (`scripts/architecture_fresh.py` fails once the workspace
version moves past the line above).

This page is the map: where things live, what talks to what, and the
rules that hold everywhere. The why of each choice is in the
[ADRs](docs/adr/); the what, in the [specification](docs/SPEC.md).

## Bird's-eye view

AI Juggler is a desktop app that runs and watches AI coding tools
(Claude Code, Codex, OpenCode, Grok Build, Kimi Code, and their desktop
apps) across several accounts, and moves a conversation from one account
or tool to another. It never sits between the user and a tool: it starts
the real, unmodified tool in the right folder, reads what the tool writes
to disk, and shows it.

```
 ┌──────────── ui/ (React, in a Tauri web view) ────────────┐
 │ screens · pages · components · lib/ipc (the only caller)  │
 └───────────────▲──────────────────────────┬────────────────┘
                 │ events                   │ invoke (ids, chosen paths)
 ┌───────────────┴──────────────────────────▼────────────────┐
 │ juggler-app: commands, features/*, state, the window       │
 └──┬─────────┬──────────┬───────────┬──────────┬────────────┘
    │         │          │           │          │
 adapters  supervisor  handover    link      account · stats · floor
    │         │          │           │
 adapter (the contract) ─┴─ core (model, storage) ─ platform (OS seams)
```

A tool is known to AI Juggler only through its adapter. Everything else
asks the adapter contract, never a tool's files or names directly.

## Codemap

- `crates/juggler-core` — the domain model (Account, App, Project,
  Session, pools, tasks), SQLite storage and its migrations, usage
  accounting, and the backup archive's format and reader (the restore's
  check and the fuzz target share it). Pure: no process, no network, no UI (its `ts` feature, on
  only in the app's build, gives the few domain types a command returns
  their TypeScript).
- `crates/juggler-platform` — the operating-system seams: who we are,
  owner-only files, local sockets, program trust, starting apps, helper
  processes. The one place macOS, Windows and Linux differ.
- `crates/juggler-adapter` — the `HarnessAdapter` contract: the sum of
  eleven capability traits of at most 12 methods each (`ToolIdentity`,
  `ToolDiscovery`, `ToolSignIn`, `ToolLaunch`, `ToolConfig`,
  `ToolInventory`, `ToolTranscripts`, `ToolQuota`, `ToolLive`,
  `ToolHandover`, `ToolRuntime`; one import: `juggler_adapter::prelude`),
  its side traits (subagents, automation, driven runtime), discovery,
  launch specs, the system tools by absolute path, the user's `PATH`.
- `crates/juggler-adapters` — one module family per tool (Claude Code and
  the Claude app, Codex, OpenCode, Grok, Kimi, ZCode, the fake harness),
  the building blocks in `common/` that name no tool (a config home's
  layout, scan, inspection, copy and adoption; the managed-folder marker;
  the version probe; bounded JSONL lines; read-only SQLite), plus neutral
  shared mechanics (transcripts, inference, quota ingest).
- `crates/juggler-supervisor` — starting and watching processes: the PTY
  and piped runtimes, the process tree and ledger, zombies, live stats.
- `crates/juggler-handover` — switching: fidelity tiers, context packs,
  lineage, project sync, redaction.
- `crates/juggler-link` — Fleet: discovery, pairing (SPAKE2), mutual-TLS
  gRPC between Macs, the machine identity.
- `crates/juggler-account` — the optional Figurehead account: the device
  token and the backup vault (keys in the Keychain).
- `crates/juggler-agentbridge` — `juggler-agent`, the tools agents in a
  pool use to message and hand tasks to each other (MCP stdio).
- `crates/juggler-floor`, `juggler-floor-render` — the Floor's model and
  its native wgpu window.
- `crates/juggler-stats` — opt-in, identifier-free usage statistics.
- Audited shims, each a handful of `unsafe` calls behind a safe API:
  `juggler-login-item`, `juggler-scap-init`, `juggler-vm-stats`,
  `juggler-window-events`.
- `crates/juggler-app` — the Tauri shell: `lib.rs` (the command registry
  `app_commands!`, start-up), `features/<feature>/` (every command and its
  logic: `commands.rs` the thin `#[tauri::command]`s, `dto.rs` the wire
  types, the logic in `mod.rs` or named files; a command is named for its
  folder, `scripts/ipc_audit.py`), `commands.rs` (only the helpers every
  feature shares: the storage lock, now, this Mac's id, finding an account
  or profile), `events.rs` (`AppEvent`, every event the UI hears), `state.rs`,
  `webview_guard.rs` and `picked.rs` (what the web view may do and hand
  over), `capabilities/` and `permissions/` (each window's grant).
- `ui/src` — `shell` (App's state in hooks, and the registries of what
  the window shows: `screens.tsx` one entry per header screen, `pages.tsx`
  an item's page, `Overlays.tsx` the sheets and dialogs), `screens` and
  `pages` (the views themselves; the inspector's tabs, Settings' panes and
  Home's cards are registries too), `components` (shared pieces),
  `lib/ipc/<feature>.ts` (every backend call and `listen`, one module per
  feature, each call through `transport.ts`'s typed `call`; `lib/bridge.ts`
  keeps only what they share: `EVENTS`,
  `hasLiveBackend`, `errorText`), `lib/use-resource.ts` (how a screen reads
  backend state), `preview` (the design preview's data, loaded lazily
  through `lib/preview-hooks.ts` only without a live backend; never in the
  app), `types/generated` (from Rust, by ts-rs, and `commands.ts`, the
  command map; never edited).
- `docs/` — the specification, ADRs, the roadmap and tasks, QA evidence.
- `scripts/` — the gates `just verify` and CI run.

## Invariants (things that are never there)

- **No tool's name or file layout outside its adapter.** Core, the app
  and the UI ask the contract (`scripts/tool_literal_lint.py`).
- **No dependency pointing outward.** The crate graph in
  `docs/plan/layering.toml` (`scripts/check_layering.py`); an adapter
  never imports a sibling tool's module.
- **No credential read, copied or sent.** Sign-in happens in the tool's
  own flow; AI Juggler checks only that one exists.
- **No proxying.** The real tools run unmodified; nothing intercepts their
  traffic.
- **No write into a user's own setup they didn't choose,** and no change
  to an original when it's copied.
- **No path, URL or program from the page.** Commands take ids, or a path
  the user picked or dropped, or one AI Juggler already knows
  (`picked.rs`, `scripts/webview_lint.py`).
- **No web view off the app's own pages,** no popups, no broad Tauri
  permission (`webview_guard.rs`, `capabilities/`).
- **No helper process with the app's whole environment**
  (`juggler_platform::process`, `scripts/spawn_lint.py`).
- **No `unsafe` outside the audited shims** (`unsafe_code = "forbid"`).
- **No OS-only assumption outside `juggler-platform`**
  (`scripts/platform_lint.py`).
- **No action addressed by "the open page".** Every action carries its
  target's id end to end.
- **No release without its whole roadmap scope** (`scripts/release_scope.py`).

## Boundaries

- **Page ↔ backend:** a command called with ids; events back, each an
  `AppEvent` emitted through `EmitEvent::emit_event` (no event by a bare
  name). The UI calls the backend only through `lib/ipc`, and `lib/ipc`
  only through `call` (ADR-0029, AJ-E2019): `types/generated/commands.ts`
  maps every registered command to its arguments and result, generated
  from the `app_commands!` registry and each command's signature by
  `crates/juggler-app/src/command_types.rs` when `cargo test` runs, so a
  renamed command or argument, or a changed result, fails `tsc`.
  `scripts/ts_types_fresh.sh` fails when the committed map is stale; a raw
  `invoke` is `transport.ts`'s own (`ui/.oxlintrc.json`). The same test
  checks the dev bridge's routes (`dev_bridge/routes.rs`) against the
  commands' argument lists.
- **App ↔ tools:** the adapter contract. A new tool is a new adapter
  module family; nothing else changes.
- **This Mac ↔ other Macs:** `juggler-link` only, mutual TLS with pinned
  keys.
- **This Mac ↔ Figurehead:** `juggler-account` only, and only when the
  user links an account.
- **Rust ↔ OS:** `juggler-platform` and the audited shims.

## Cross-cutting concerns

- **Errors** are typed (`AppError`, `CoreError`) and said in plain words;
  a secret never prints (`juggler_core::redacted::Redacted`).
- **Threads:** commands are async and do their work off the main thread
  (`ipc::blocking`); lock order is storage, never together with Fleet.
- **Storage** is one SQLite database, migrated forward on open, every
  connection in defensive mode; a restored database is checked before
  use (`storage::foreign`). One writer lock (`AppState::storage`), held
  for the rows, never for the work (ADR-0030: a hold is one frame at
  most); a repository with its own hot writer has its own connection.
- **Testing:** the fake harness drives lifecycles; the visual and
  accessibility gates cover every gallery state; the scope gate covers
  releases. See `docs/plan/QUALITY.md`.
- **Cross-platform:** macOS today, Windows in 0.6, Linux in 0.7 (D-52);
  every OS difference behind a seam.
