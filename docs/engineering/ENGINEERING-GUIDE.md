# Engineering guide

How code in this repo is written: patterns, rules and limits for every PR. It turns ADR-0019 (extension architecture) and the research in `docs/research/engineering-standards.md` (43 primary sources, 2026-10-21) into rules.

**MUST** means CI or review blocks the PR. **SHOULD** is the default; explain any deviation in the PR. Each rule names its enforcement. "Proposed" marks a check that E20 is building. Until it lands, review enforces the rule.

Read with: [ADR-0019](../adr/0019-extension-architecture.md), [the security guidelines](SECURITY-GUIDELINES.md), [QUALITY.md](../plan/QUALITY.md).

### C.1 Architecture

**A1 MUST: Dependencies point inward, per the allowed graph.**
```
ui ─IPC─▶ juggler-app (composition root + IPC edge)
juggler-app ─▶ feature crates/modules ─▶ juggler-adapter (ports) ◀─ juggler-adapters (adapters)
everything ─▶ juggler-core (domain; knows no tool, no Tauri, no UI)
juggler-account, juggler-stats, juggler-link, juggler-supervisor, juggler-handover ─▶ core, adapter only
```
*Why:* this is the rust-analyzer invariant style: name what a crate must *not* know
[S3], and keep ports and adapters apart [S16]. *Enforced:* proposed
`scripts/check_layering.py`, which reads `cargo metadata --format-version 1 --no-deps`
and compares it with `docs/plan/layering.toml`, in `just verify` and CI. It closes G4.

**A2 MUST: No tool-id literal outside `juggler-adapters`, adapter data files, generated
code or tests.** The UI and the commands read `ToolSpec` from `tools_catalog`
(ADR-0019 §1). *Why:* each literal is a branch that every new tool must find, so it
costs N×. *Enforced:* proposed `scripts/tool_literal_lint.py` with `scripts/tool_literals.allow`, an allowlist of
today's 155 sites that **only shrinks** (the script fails if a listed site disappears
without being removed from the file). Run in `just verify` and CI.

**A3 MUST: Commands are thin.** A command only does four things: parse and validate
the wire arguments into domain types, call one service method, map the result to a DTO,
and map the error to `AppError`. No SQL, no loops over domain data, no tool knowledge.
*Why:* thin handlers [S17]. Logic in commands can't be reached from tests, Fleet or the
agent bridge without going through the IPC layer. *Enforced:* review, plus a proposed
`scripts/ipc_audit.py` that fails any `#[tauri::command]` function body longer than 40
lines.

**A4 MUST: Features are modules (ADR-0019 §3).**
```
crates/juggler-app/src/features/<feature>/
  mod.rs        // pub use of the service trait + DTOs; nothing else is pub(crate)+
  service.rs    // trait <Feature>Service + impl; owns the feature's state
  repo.rs       // storage behind a trait; migrations named <feature>_NNN
  commands.rs   // thin #[tauri::command]s, all named <feature>_<verb>
  events.rs     // variants of the one typed event enum
  dto.rs        // #[derive(Serialize, TS)] *Info types
```
Other modules use `features::<x>::{Service, dto}` only, never `features::<x>::repo` or
internals (`pub(crate)` is the default inside a feature, and `pub(in crate::features::x)`
where narrower). *Enforced:* proposed module-import rule in `check_layering.py` (grep
for `features::<other>::(repo|service_impl|…)`), plus review.

**A5 MUST: One explicit registry, no link-time magic.** Adapters, features, MCP tools
and UI contributions register in one function each (`registry::builtin()`), with no
`inventory` or `linkme`. *Why:* life-before-main, unspecified order, and
platform-dependent and linker-dependent registration [S19][S20][S21]. The set is
closed and in-repo. *Enforced:* `cargo deny` `[bans] deny = [{ name = "inventory" },
{ name = "linkme" }]` (proposed). The registry test asserts the expected ids.

**A6 MUST: Shared adapter mechanics live in neutral modules.** Put them in
`juggler-adapters/src/common/{layout.rs, probe.rs, home.rs, jsonl.rs, sqlite.rs}`. A
tool module never imports from a sibling tool module. *Why:* ADR-0019 §2, and it keeps
adapter #3 from copying Claude's. *Enforced:* proposed grep check: `use crate::(claude|
codex|zcode)_` inside another tool's module fails.

**A7 SHOULD: Focused traits, object-safe, one job each.** Split `HarnessAdapter` into
`ToolSpec` (data) plus `Discover`, `Launch`, `Transcripts`, `Quota` and `Handover`
capabilities, each with 12 methods or fewer, obtained from the registry as
`Arc<dyn …>`. Use generics inside a crate (helpers and the conformance kit) and
`dyn` at the registry seam. *Why:* C-OBJECT and C-GENERIC [S4], the trade-offs in
[S13], and ADR-0019's "one mega-trait blurs ownership". *Enforced:* review. The
conformance kit compiles against each trait.

### C.2 Errors

**E1 MUST: One thiserror enum per crate (or per feature) in library code. `anyhow` is
allowed only in `bin/`, `examples/`, `xtask` and tests.** Domain enums describe
everything that can go wrong. Adapters map foreign errors into them, and the IPC edge
maps them to `AppError` → `{ kind, message }`. *Why:* [S14][S17][S6]. *Enforced:*
proposed `cargo deny` ban of `anyhow` with wrappers limited to the bin, or a grep in
`check_layering.py`. Review.

**E2 MUST: Never turn an error into a default value without a `tracing::warn!`, and
never in a command, where the UI must get `Err`.** *Why:* silent degradation hides
faults (G11). *Enforced:* review, plus clippy `let_underscore_must_use` and
`map_err_ignore` (restriction lints, cherry-picked [S26]).

**E3 MUST: Every lint exception uses `#[expect(lint, reason = "…")]`, not `#[allow]`.**
*Enforced:* clippy `allow_attributes = "warn"` and `allow_attributes_without_reason =
"warn"` in `[workspace.lints.clippy]`. Set `allow-unwrap-in-tests`,
`allow-expect-in-tests` and `allow-panic-in-tests = true` in `clippy.toml` to remove the
eight `allow(clippy::pedantic)` test blocks.

```rust
// crates/juggler-app/src/features/sync/error.rs
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("peer {peer} is not paired")]
    NotPaired { peer: MachineId },
    #[error(transparent)]
    Storage(#[from] juggler_core::CoreError),
}
impl From<SyncError> for AppError {
    fn from(e: SyncError) -> Self {
        match e {
            SyncError::NotPaired { .. } => AppError::Validation { subject: "sync", message: e.to_string() },
            SyncError::Storage(inner) => AppError::Core(inner),
        }
    }
}
```

### C.3 Async, threading and state

**T1 MUST: A command that touches the filesystem, SQLite, the network or processes is
`async`, and its blocking work runs in `spawn_blocking`.** A sync command is allowed
only for pure, sub-millisecond work, and it carries an `// ipc: main-thread (pure)`
marker. *Why:* sync commands run on the main thread [S6] (G1). *Enforced:* proposed
`scripts/ipc_audit.py`, which fails any non-async `#[tauri::command]` without the marker,
seeded with an allowlist of today's commands that only shrinks.

```rust
#[tauri::command]
pub async fn cleanup_inventory(state: State<'_, CleanupState>) -> AppResult<CleanupInventoryInfo> {
    let service = state.service.clone();                    // Arc<dyn CleanupService>
    tauri::async_runtime::spawn_blocking(move || service.inventory())
        .await
        .map_err(|_| AppError::TaskJoin)?                    // new variant
        .map(CleanupInventoryInfo::from)
        .map_err(AppError::from)
}
```

**T2 MUST: Per-feature state.** Register it with `app.manage(FeatureState)` under a type
alias. Never hold a `std::sync::Mutex` guard across `.await`, and use `tokio::sync::Mutex`
only when you must [S7]. **SHOULD:** replace the single `Mutex<Storage>` with
per-repository connections or a small pool (an ADR, since it is a new dependency). *Why:*
the god-object `AppState` couples every feature, and one lock serialises all reads.
*Enforced:* review. `AppState` gains no new fields (proposed grep check on its field
count).

**T3 MUST: Per-call progress uses `tauri::ipc::Channel<T>`. App-wide notifications use
the one typed event enum.** Event names are constants generated from that enum, never
string literals at emit sites. *Why:* [S6]. Events aren't type safe. *Enforced:*
proposed grep: `.emit("` outside `events.rs` fails.

**T4 MUST: `juggler-core` stays sync and pure. Threads are spawned only in
`juggler-app`/`juggler-supervisor`, with a name and a `poll:` note** (the existing
polling audit). *Enforced:* the existing `polling_audit.py`, plus a proposed
`disallowed-methods` entry for `std::thread::spawn` in core (per-crate `clippy.toml`).

### C.4 IPC and typed bindings

**I1 MUST: Wire types are DTOs (`*Info`, `*Request`) with `#[derive(Serialize, TS)]`.
Domain types never derive `TS`.** *Why:* the rust-analyzer serialisation boundary [S3]
and thin adapters [S17]. *Enforced:* review, plus a grep that `ts(export)` appears only in
`dto.rs`/`commands*.rs`.

**I2 MUST: The generated bindings are fresh.** *Enforced:* proposed CI step after
`cargo test`: `git diff --exit-code -- ui/src/types/generated` [S23] (G7).

**I3 MUST: Every `invoke` name in the UI is a registered command, and every registered
command has a caller or a dated allowlist entry.** *Enforced:* proposed
`scripts/ipc_audit.py` . **SHOULD, later:** generate typed command wrappers (tauri-specta
when 2.0 is stable [S24], or a small in-repo generator) so names and arguments are
compile-checked. Record that choice in an ADR.

**I4 MUST: Command names are `<feature>_<verb>` and unique. Renames keep an alias only
with a removal version in its doc comment.** *Why:* names are global [S6]. *Enforced:*
`ipc_audit.py` checks the prefix against the feature folder.

**I5 SHOULD: When a second webview appears (menu bar, mini window), restrict app
commands per window** with `AppManifest::commands` and capabilities [S25].

```ts
// ui/src/lib/ipc/sync.ts — one file per feature, thin and typed (generated later)
import { invoke } from "@tauri-apps/api/core";
import type { SyncStatusInfo } from "../../types/generated/SyncStatusInfo";
export const syncStatus = (): Promise<SyncStatusInfo> => invoke("sync_status");
```

### C.5 Configuration

**K1 MUST: Environment variables are read only in `juggler-app/src/env.rs` (and the
edition module) and in adapters, for the tool's own variables. Everything else receives
typed config.** *Enforced:* `clippy.toml` `disallowed-methods = [{ path =
"std::env::var", reason = "read through env.rs" }]`, with
`#[expect(clippy::disallowed_methods, reason = …)]` at the sanctioned sites [S27].

**K2 MUST: Settings are validated newtypes at load time**, parsed rather than
re-validated at each use [S10]. Storage migrations are named per feature in the one
sequence (ADR-0019).

### C.6 Logging and tracing

**L1 MUST: Library crates depend on `tracing` only. `juggler-app` installs the one
subscriber (rolling file in the data dir, redaction layer). No `println!`/`eprintln!`
outside `bin/`, `examples/` and tests.** Use spans with `#[instrument(skip_all, fields(
instance = %id))]` around commands and long jobs. *Why:* [S15], and QUALITY §2 already
requires it (G4). *Enforced:* `[workspace.lints.clippy] print_stdout = "warn"`,
`print_stderr = "warn"` and `dbg_macro = "warn"` (restriction, cherry-picked), with
`expect` at the few fatal-startup sites.

### C.7 Testing

**Q1 MUST: Push each test as low as it can go** [S28]. Domain logic gets unit tests and
proptest. Parsers get fixtures, golden files and fuzzing. Services are tested against
fake repositories. Commands get a few IPC-level tests. Playwright covers the critical
journeys only.

**Q2 MUST: The adapter conformance kit is one generic suite run for every registered
adapter.** A new tool adds fixtures, not new test code.
```rust
// crates/juggler-adapters/tests/conformance.rs
fn conformance(adapter: &dyn HarnessAdapter, fixtures: &Path) {
    assert!(!adapter.verified_against().is_empty());
    for case in fixture_cases(fixtures) { check_transcript_roundtrip(adapter, &case); }
}
#[test] fn every_builtin_adapter_conforms() {
    for a in registry::builtin().all() { conformance(&*a, &fixtures_for(a.id())); }
}
```

**Q3 MUST: There is a test that renders first run from a catalog with an unknown fake
tool** (ADR-0019). The UI must need no change for a new tool.

**Q4 SHOULD: Keep the existing golden-file and `AIJ_BLESS` helper as the snapshot
mechanism.** Adopt insta [S30] only by ADR. Inline `#[cfg(test)]` modules over ~300
lines move to `<module>/tests.rs`.

**Q5 MUST: Coverage thresholds in QUALITY §5 fail CI** (drop `continue-on-error`, or
change QUALITY to say the numbers are gate-time only). A scheduled fuzz workflow runs
`fuzz/` targets nightly (G4).

### C.8 Lints and formatting (Rust)

Add to `Cargo.toml`:
```toml
[workspace.lints.rust]
unsafe_code = "forbid"
missing_docs = "deny"
unreachable_pub = "warn"          # app crates: finds pub that is only crate-visible
[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }
unwrap_used = "warn"
expect_used = "warn"
panic = "warn"
todo = "warn"
unimplemented = "warn"
allow_attributes = "warn"
allow_attributes_without_reason = "warn"
print_stdout = "warn"
print_stderr = "warn"
dbg_macro = "warn"
let_underscore_must_use = "warn"
map_err_ignore = "warn"
```
Add a `clippy.toml`:
```toml
too-many-lines-threshold = 100
too-many-arguments-threshold = 7
max-struct-bools = 3
allow-unwrap-in-tests = true
allow-expect-in-tests = true
allow-panic-in-tests = true
disallowed-methods = [
  { path = "std::env::var", reason = "read through juggler-app::env" },
]
```
Centralise dependency versions in `[workspace.dependencies]` and use
`dep = { workspace = true }` in members [S5]. Keep `-D warnings` in CI, not in source
[S31]. Run rustdoc with `-D warnings` (exists).

### C.9 TypeScript and React

**U1 MUST: tsconfig additions** (`strict` is already on): `noUncheckedIndexedAccess`,
`noImplicitReturns`, `noImplicitOverride`, `verbatimModuleSyntax`. **SHOULD:**
`exactOptionalPropertyTypes` (more churn against ts-rs `Option` fields) [S32].

**U2 MUST: An ESLint flat config, with `pnpm lint` in CI.** It uses typescript-eslint
`strict-type-checked`, pinned since that set can change [S33], plus `react-hooks` [S34]
and `jsx-a11y`. Key rules: `no-explicit-any`, `no-unsafe-*`, `no-floating-promises`,
`no-non-null-assertion` (warn), `react-hooks/rules-of-hooks` (error),
`react-hooks/exhaustive-deps` (error), `max-lines` and `max-lines-per-function` (see
limits) [S42], `complexity: 15`, and `no-restricted-syntax` for tool-id literals.
Import boundaries (`no-restricted-imports`): only `src/lib/ipc/**` may import
`@tauri-apps/api`; `components/**` never imports `screens/**`; `screens/*` never import
each other; `preview/**` is never imported by production code.

```js
// ui/eslint.config.js (sketch)
import tseslint from "typescript-eslint";
import hooks from "eslint-plugin-react-hooks";
import a11y from "eslint-plugin-jsx-a11y";
export default tseslint.config(
  ...tseslint.configs.strictTypeChecked,
  a11y.flatConfigs.recommended,
  { plugins: { "react-hooks": hooks },
    rules: {
      "react-hooks/rules-of-hooks": "error",
      "react-hooks/exhaustive-deps": "error",
      "max-lines": ["error", { max: 400, skipBlankLines: true, skipComments: true }],
      "max-lines-per-function": ["warn", { max: 150, skipBlankLines: true, skipComments: true }],
      complexity: ["warn", 15],
      "no-restricted-syntax": ["error",
        { selector: "Literal[value=/^(claude-code|codex|zcode)$/]", message: "Read ToolSpec from tools_catalog (ADR-0019)" }],
      "no-restricted-imports": ["error", { patterns: [
        { group: ["@tauri-apps/api/*"], message: "Only src/lib/ipc may call the backend" }] }],
    } },
  { files: ["src/lib/ipc/**"], rules: { "no-restricted-imports": "off" } },
  { files: ["**/*.test.tsx", "tests/**", "src/preview/**"], rules: { "no-restricted-syntax": "off" } },
);
```

**U3 MUST: Split the backend seam.** `lib/ipc/<feature>.ts` holds thin typed calls (the
only `invoke` sites). `src/preview/` holds fixtures and mocks, loaded only when
`!hasLiveBackend()` through a dynamic `import()`, so they leave the production bundle.
One typed `PreviewHooks` interface in `global.d.ts` replaces the
`window as unknown as {…}` casts. *Enforced:* the import boundaries above, plus
`knip` for unused exports [S40].

**U4 MUST: Components do one thing** [S22]. Derived values are computed during render.
Effects exist only to sync with external systems [S35]. A component with more than 8
`useState` or more than 4 `useEffect` moves its logic into a custom hook or a
`useReducer` [S36]. Backend resources are read through one hook built on
`useSyncExternalStore` (subscribe to the typed event, refetch, cancel stale responses).

```ts
// ui/src/lib/ipc/useResource.ts — the one way screens read backend state
export function useResource<T>(key: string, load: () => Promise<T>, event?: string) {
  const store = getStore(key, load, event);          // module-level cache + listeners
  return useSyncExternalStore(store.subscribe, store.snapshot);
}
```

**U5 MUST: The shell renders from registries (ADR-0019 UI contributions).** Screens,
inspector tabs, settings sections and Home cards are declared in
`src/shell/registry.ts`, not as `if (screen === …)` chains in `App.tsx`. Tool display
names, marks and sign-in words come from `tools_catalog`. Delete the tables in
`App.tsx:159` and `providers.ts`.
```ts
export const screens: readonly ScreenContribution[] = [
  { id: "usage", title: "Usage", icon: "chart", load: () => import("../screens/Usage") },
  // features append here; App.tsx maps over the list
];
```

**U6 MUST: Accessibility at authoring time.** `jsx-a11y` lint, and APG patterns for
dialogs, tabs and listboxes [S37]. Colour is never the only signal (existing rule). The
existing axe and keyboard gate stays the release check.

### C.10 Limits

| Limit | Value | Level | Enforcement |
|---|---|---|---|
| Rust file, non-test lines | ≤ 800 SHOULD, ≤ 1,500 MUST for new or grown files | MUST | proposed `scripts/size_budget.py` with an allowlist that only shrinks (today 17 files > 1,000). Generated files exempt |
| Rust function | ≤ 100 lines; ≤ 7 args | MUST | clippy `too_many_lines`, `too_many_arguments` (exists), `expect` with reason |
| Tauri command body | ≤ 40 lines | MUST | `ipc_audit.py` |
| Trait | ≤ 12 methods (required + provided) | SHOULD | review |
| Struct bools | ≤ 3 (C-CUSTOM-TYPE: use enums) | MUST | clippy `struct_excessive_bools` |
| TS file | ≤ 400 lines | MUST for new files; allowlist for old | ESLint `max-lines` |
| TS function/component | ≤ 150 lines (component JSX ≤ ~200 incl. markup) | SHOULD | `max-lines-per-function` |
| Component state | ≤ 8 `useState`, ≤ 4 `useEffect` | SHOULD | review (or a custom lint later) |
| Cyclomatic complexity | ≤ 15 (TS); Rust `cognitive_complexity` stays off (nursery) | SHOULD | ESLint `complexity` |
| Tool-id literals outside adapters | 0 (allowlist reaches 0 by 0.2) | MUST | `tool_literal_lint.py`, ESLint `no-restricted-syntax` |
| New duplication | 0 new clones ≥ 50 tokens | MUST | `jscpd --fail-on-new-clones` [S41] |
| Unused deps / exports | 0 | MUST | `cargo machete` [S38]; `knip` [S40]; `cargo +nightly udeps` monthly [S39] |
| PR diff | ≤ ~400 lines (exists) | SHOULD | review; mechanical moves exempt and labelled |

### C.11 Documentation

**D1 MUST: Keep an `ARCHITECTURE.md` at the root in matklad's style** [S2]: bird's eye
view, a codemap with names and no links, invariants stated as absences ("`juggler-core`
knows no tool id", "the UI never branches on a tool", "only `juggler-app` knows Tauri",
"only adapters read a tool's files"), layer boundaries (IPC, adapter ports, storage) and
cross-cutting concerns (errors, tracing, redaction, guardrails). Revisit it at each
milestone gate. *Enforced:* a gate-packet checkbox, plus a `research_staleness.sh`-style
date check.

**D2 MUST: Every crate has a `//!` doc with its role and what it must not know.**
(The crate docs already exist. Add the "must not know" line.) A crate README is
needed only for non-obvious usage. **D3:** ADRs stay the record of *why*. When QUALITY.md
and the code disagree, fix one of them in the same PR.

---

## Code-review checklist


Paste into `.github/PULL_REQUEST_TEMPLATE.md` (and remove its duplicated "Visual
baselines" section).

**Structure**
- [ ] The change lives in the right feature module or crate. There are no new items in
      `commands.rs`, no new `AppState` fields, and no new `bridge.ts` functions outside
      `lib/ipc/<feature>.ts`.
- [ ] Dependency direction and module-internal rules hold (A1, A4). No sibling-adapter
      imports (A6).
- [ ] There is no tool-id literal or per-tool branch outside adapters. New tool facts
      go in `ToolSpec` (A2).
- [ ] No file or function grew past the limits (C.10), or the PR explains the expect
      and the reason.

**Behaviour and correctness**
- [ ] Commands are thin and `async` when they do I/O (A3, T1). No guard is held across
      `.await`.
- [ ] Errors are typed. No silent default, `unwrap` or `expect` in production paths
      (E1–E3). The UI shows `Err` in words.
- [ ] Malformed external input degrades with a `tracing::warn!` and never panics.

**Contracts**
- [ ] DTOs, not domain types, cross IPC. The generated bindings are committed and fresh
      (I1, I2).
- [ ] New and renamed commands follow `<feature>_<verb>`, and any alias has a removal
      version (I4).
- [ ] Events go through the typed event enum, and per-call progress uses a `Channel` (T3).

**Tests**
- [ ] The test sits at the lowest level that proves the behaviour (Q1). Fixtures grew if
      a parser changed. The conformance kit passes for every adapter.
- [ ] UI logic lives in hooks or pure functions with vitest tests. Playwright covers only
      the journey.

**UI**
- [ ] No effect is used for derived state. Hooks follow the rules (ESLint clean). Each
      component has one job.
- [ ] a11y: names, roles and keyboard path work, colour is never the only signal, and
      the axe gate is green.

**Hygiene**
- [ ] No dead code, unused dependencies or exports (machete and knip clean). No new
      clones (jscpd).
- [ ] Docs, CHANGELOG, ADR (if a decision was made) and ARCHITECTURE.md (if an invariant
      changed) are updated.
- [ ] A duplicated path or "just this once" shortcut is a finding even when tests pass
      (ADR-0019 §5).

---


## Sources (all accessed 2026-10-21)

- [S1] matklad, "Large Rust Workspaces" — https://matklad.github.io/2021/08/22/large-rust-workspaces.html
- [S2] matklad, "ARCHITECTURE.md" — https://matklad.github.io/2021/02/06/ARCHITECTURE.md.html
- [S3] rust-analyzer, Architecture — https://rust-analyzer.github.io/book/contributing/architecture.html
- [S4] Rust API Guidelines, Checklist — https://rust-lang.github.io/api-guidelines/checklist.html
- [S5] The Cargo Book, Workspaces — https://doc.rust-lang.org/cargo/reference/workspaces.html
- [S6] Tauri 2, Calling Rust from the Frontend — https://v2.tauri.app/develop/calling-rust/
- [S7] Tauri 2, State Management — https://v2.tauri.app/develop/state-management/
- [S8] Tauri 2, Plugin Development — https://v2.tauri.app/develop/plugins/
- [S9] Tauri 2, Inter-Process Communication — https://v2.tauri.app/concept/inter-process-communication/ ; Tauri Architecture — https://v2.tauri.app/concept/architecture/
- [S10] The Rust Book, ch. 9.3 "To panic! or Not to panic!" — https://doc.rust-lang.org/book/ch09-03-to-panic-or-not-to-panic.html
- [S11] The Rust Book, ch. 7.2 Modules — https://doc.rust-lang.org/book/ch07-02-defining-modules-to-control-scope-and-privacy.html
- [S12] The Rust Reference, Visibility and privacy — https://doc.rust-lang.org/reference/visibility-and-privacy.html
- [S13] The Rust Book, ch. 18.2 Trait objects — https://doc.rust-lang.org/book/ch18-02-trait-objects.html
- [S14] dtolnay/anyhow README — https://github.com/dtolnay/anyhow
- [S15] `tracing` crate docs — https://docs.rs/tracing/latest/tracing/
- [S16] Alistair Cockburn, Hexagonal Architecture — https://alistair.cockburn.us/hexagonal-architecture/
- [S17] How To Code It, "Master hexagonal architecture in Rust" — https://www.howtocodeit.com/articles/master-hexagonal-architecture-rust
- [S18] Michael F. Bryan, "Plugins in Rust" — https://adventures.michaelfbryan.com/posts/plugins-in-rust/
- [S19] dtolnay/inventory README — https://github.com/dtolnay/inventory
- [S20] dtolnay/linkme README — https://github.com/dtolnay/linkme
- [S21] Static-archive linker drops unreferenced registration objects: Clang SSAF "Force-Linker Headers" — https://clang.llvm.org/docs/ScalableStaticAnalysisFramework/developer-docs/ForceLinkerHeaders.html ; Rust internals "Global Registration (a kind of pre-RFC)" — https://internals.rust-lang.org/t/global-registration-a-kind-of-pre-rfc/20813/4
- [S22] React, Thinking in React — https://react.dev/learn/thinking-in-react
- [S23] Aleph-Alpha/ts-rs README — https://github.com/Aleph-Alpha/ts-rs
- [S24] specta-rs/tauri-specta README — https://github.com/specta-rs/tauri-specta
- [S25] Tauri 2, Capabilities — https://v2.tauri.app/security/capabilities/
- [S26] Clippy, lint groups — https://doc.rust-lang.org/clippy/lints.html
- [S27] Clippy, lint configuration — https://doc.rust-lang.org/clippy/lint_configuration.html
- [S28] Ham Vocke (martinfowler.com), "The Practical Test Pyramid" — https://martinfowler.com/articles/practical-test-pyramid.html
- [S29] Proptest book, Introduction — https://proptest-rs.github.io/proptest/intro.html
- [S30] Insta docs — https://insta.rs/docs/
- [S31] Rust Design Patterns, anti-pattern `#![deny(warnings)]` — https://rust-unofficial.github.io/patterns/anti_patterns/deny-warnings.html
- [S32] TypeScript TSConfig reference (strict and related flags) — https://www.typescriptlang.org/tsconfig/#strict
- [S33] typescript-eslint, Shared configs — https://typescript-eslint.io/users/configs/
- [S34] React, Rules of Hooks — https://react.dev/reference/rules/rules-of-hooks
- [S35] React, You Might Not Need an Effect — https://react.dev/learn/you-might-not-need-an-effect
- [S36] React, Scaling Up with Reducer and Context — https://react.dev/learn/scaling-up-with-reducer-and-context
- [S37] W3C WAI-ARIA Authoring Practices Guide — https://www.w3.org/WAI/ARIA/apg/
- [S38] bnjbvr/cargo-machete README — https://github.com/bnjbvr/cargo-machete
- [S39] est31/cargo-udeps README — https://github.com/est31/cargo-udeps
- [S40] Knip — https://knip.dev/
- [S41] kucherenko/jscpd README — https://github.com/kucherenko/jscpd
- [S42] ESLint `max-lines` — https://eslint.org/docs/latest/rules/max-lines
- [S43] obi1kenobi/cargo-semver-checks README — https://github.com/obi1kenobi/cargo-semver-checks

### Caveats

- The tool-literal, clone and function-length counts come from heuristic scripts (regex
  with brace matching, and test modules stripped by `#[cfg(test)]` blocks). Treat them
  as ±10 %.
- "~12 uncalled commands" means no caller under `ui/src`, `ui/tests` or `scripts`. A
  few may be reached through Fleet remote commands or Rust tests, so check each before
  deleting it.
- tauri-specta is still at a release candidate (`2.0.0-rc.21` per its README), so I21
  is deliberately a SHOULD behind an ADR.
- A Tauri in-app plugin per feature (namespaced `plugin:<feature>|cmd`, per-feature
  permissions) is a viable alternative to A4's single handler list. It costs a
  permission manifest per feature and a rename of every UI call. That choice belongs in
  the ADR-0019 follow-up and is not decided here.
