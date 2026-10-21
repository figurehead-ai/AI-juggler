# Engineering standards for AI Juggler: research, gap analysis, draft guide

Prepared 2026-10-21. Read-only study of `main` at `0f4dc8e3`, plus ADR-0019 from
`origin/docs/adr-0019-extension-architecture` (PR #110). All sources accessed 2026-10-21.
The measurement scripts used were run once for this report
(`tool_literals.py`, `dupes.py`, `fnlen.py`). They are prototypes for the proposed lints.

Contents

- (a) Sourced summary of practices
- (b) The repo today, with gaps ranked by risk
- (c) Draft ENGINEERING GUIDE (patterns, MUST/SHOULD rules, limits, enforcement)
- (d) Code-review checklist
- (e) Prioritised enforcement steps (S/M/L)
- Sources

---

## (a) Sourced summary of practices

### 1. Workspace and crate layering, dependency direction

- **Flat `crates/*` layout and a virtual root manifest.** For projects of roughly
  10k–1M lines, matklad recommends putting every crate one level deep under `crates/`
  and making the root a virtual manifest. He says a flat list makes "adding or splitting
  the crates is trivial", and that nested trees decay. Each crate's name should match
  its folder. Internal crates that are never published use `version = "0.0.0"`, and
  crates that are published with a real semver API go in a separate `libs/` folder, so
  it is easy to check that they don't depend on `crates/`. Automation is written in
  Rust as an `xtask` crate, not in ad-hoc scripts. [S1]
- **Name the API boundaries and the absences.** rust-analyzer's architecture doc
  states its invariants as things a crate does *not* know: "`syntax` crate is
  completely independent from the rest of rust-analyzer. It knows nothing about salsa
  or LSP"; "`base-db` knows nothing about cargo"; "`rust-analyzer` is the only crate
  that knows about LSP and JSON serialization". The core crates "don't interact with
  the outside world and thus can't fail", and only the edge does I/O. The types below
  the IDE layer are "not serializable by design", so clients that need IPC define their
  own wire formats. [S3]
- **Ports and adapters.** Cockburn's goal is to let an application "equally be driven
  by users, programs, automated test or batch scripts", isolated from devices and
  databases. A port "identifies a purposeful conversation". Driving (primary) adapters
  call in, and driven (secondary) adapters are called out to. [S16] In Rust, ports are
  traits (repository, service), adapters implement them, `main` composes everything,
  and handlers stay thin: "Validations should be defined by the domain, not request
  handlers"; "Always separate your public errors from their domain representations".
  [S17]
- **Tauri's own shape.** The webview frontend and the Rust core communicate only by
  message passing. Commands work like an FFI (request/response), and events are
  fire-and-forget notifications. [S6][S9]

### 2. Module and feature boundaries

- Everything in Rust is private by default. `pub(crate)`, `pub(super)` and
  `pub(in path)` narrow visibility to a crate, the parent module or an ancestor
  module. [S12] The Rust Book says modules group related definitions so readers
  "navigate the code based on the groups". [S11]
- React's *Thinking in React* applies the same idea to UI: "a component should ideally
  only be concerned with one thing. If it ends up growing, it should be decomposed into
  smaller subcomponents." State is kept minimal and DRY, everything derivable is
  computed, and state lives at the closest common owner. [S22]

### 3. Traits and adapters, services and repositories, registries and plugins

- **Generics or trait objects.** Generics give static dispatch, a homogeneous type and
  inlining. `dyn Trait` gives heterogeneous collections and lets new types join at
  runtime, at the cost of a vtable lookup that "prevents the compiler from choosing to
  inline". A trait must stay *dyn compatible* to be used as an object. [S13] The API
  guidelines add C-OBJECT (keep traits object-safe if they may be used as objects),
  C-GENERIC, C-NEWTYPE, C-CUSTOM-TYPE (no `bool` arguments) and C-SEALED. [S4]
- **Registries.** An in-process plugin registry holds trait objects keyed by id. The
  *registrar pattern* has each plugin call `register_*` on a registrar the host passes
  in. Dynamic loading of Rust plugins is hazardous because "Rust doesn't have a stable
  ABI". Vtables must not outlive their library, and allocators must match. [S18]
- **Distributed registration (`inventory`, `linkme`).** `inventory` runs
  life-before-main constructors. Its `collect!` must live in the crate that defines the
  plugin type, and "iteration order … is unspecified". On unsupported platforms, "no
  plugins have been registered". [S19] `linkme` uses linker sections and needs no
  runtime initialisation, but only on the platforms listed in its support table. [S20]
  Both depend on the linker keeping objects that nothing references. Static-archive
  linkers drop object files that satisfy no undefined symbol, and the usual fixes are
  anchor symbols, whole-archive linking, or explicit registration. [S21] An app with a
  closed set of in-repo adapters gets nothing from this magic, and explicit
  registration in one function is simpler and testable.
- **Tauri plugins as feature modules.** A Tauri 2 plugin is a `Builder` with its own
  `setup`/`on_event` hooks, managed state, and commands invoked as `plugin:<name>|cmd`.
  Its commands are *denied by default* and need permissions generated from `build.rs`.
  [S8] That makes a plugin a heavier but well-isolated unit, best for features you would
  ship or reuse on their own.

### 4. Error handling

- Default to `Result`. Panic only for broken invariants, contract violations, tests and
  prototypes. Encode validity in types, as the Book's `Guess` newtype with a private
  field does. [S10]
- Use **thiserror** when callers need to act on precise failures: "a library that wants
  to design your own dedicated error type(s) so that on failures the caller gets
  exactly the information that you choose". Use **anyhow** when "you don't care what
  error type your functions return". `.context()` adds the troubleshooting story. [S14]
- Errors per layer: each domain operation has an enum that is "a complete description
  of everything that can go wrong". Adapters map library errors into domain variants,
  and the edge maps domain errors to a public shape. [S17]
- In Tauri, "everything returned from commands must implement serde::Serialize,
  including errors". For larger apps the docs recommend a thiserror type with a manual
  `Serialize` impl (for example `{ kind, message }`). [S6]

### 5. Async, threading, and state in Tauri

- "Commands without the async keyword are executed on the main thread unless defined
  with `#[tauri::command(async)]`." Async commands run on `async_runtime::spawn`.
  Borrowed arguments such as `State<'_, T>` in async commands require returning a
  `Result`. [S6]
- State is registered with `app.manage(...)`. A std `Mutex` is fine and often preferred,
  and an async mutex is only needed when a guard is held across `.await`. `Arc` is not
  needed because Tauri wraps state itself. A mismatched `State<T>` type fails at runtime
  ("state not managed"), so the docs recommend type aliases. [S7]
- For streaming, "The Tauri channel is the recommended mechanism for streaming data".
  Events "are not type safe, are always async, cannot return values and only supports
  JSON payloads". [S6]

### 6. IPC command design and typed bindings

- Commands should be `pub` in modules and referenced by path. Command names are global,
  so they must be unique across modules. There is only one `generate_handler!` list, and
  a second `invoke_handler` call replaces the first. [S6]
- **ts-rs** generates TS types from Rust types. `#[ts(export)]` "generates a test which
  writes the bindings", so they are written by `cargo test`. It does *not* type command
  names or argument lists. Its README gives no drift check, so CI must diff the
  regenerated output. [S23]
- **tauri-specta** adds "completely typesafe Tauri commands" and typed events for
  Tauri 2 + Specta 2. Its docs point at `2.0.0-rc.21`, still a release candidate. [S24]
- **Capabilities.** App commands registered with `invoke_handler` are allowed for
  *every* window by default. They can be restricted with
  `tauri_build::AppManifest::new().commands(...)` in `build.rs`. [S25]

### 7. Configuration

Rust's general guidance applies: parse once at the edge into validated types (C-VALIDATE,
newtypes) [S4][S10]. Clippy's `disallowed-methods` can confine raw reads such as
`std::env::var` to one module ("use the config module instead" is the docs' own
example). [S27]

### 8. Logging and tracing

`tracing` distinguishes spans ("a period of time") from events ("a moment in time"),
records structured `key = value` fields, and offers `#[instrument]`. "Libraries should
link only to the `tracing` crate", and "libraries should *not* call
`set_global_default()`". Only the executable installs a subscriber. [S15]

### 9. Testing pyramid, fixtures, property tests

- "Write lots of small and fast unit tests", fewer integration tests at real I/O
  boundaries, and "very few high-level tests". "Push your tests as far down the test
  pyramid as you can", and avoid the "test ice-cream cone". Contract tests catch broken
  interfaces between a provider and its consumers. [S28]
- rust-analyzer's tests "are data driven and do not test the API", depend on no
  external resources, and are snapshot-checked. [S3]
- Property tests check that properties hold "for all inputs", and failing inputs are
  "automatically reduced to a *minimal* test case". They complement unit tests rather
  than replacing them. [S29]
- Insta is the ecosystem's snapshot tool, with a review workflow (`cargo insta`). [S30]

### 10. Lints and formatting

- Clippy groups: `correctness` is deny-by-default, and `#[allow]` on it is "not
  recommended". `suspicious`, `complexity` and `perf` should be fixed rather than
  allowed. `pedantic` can be enabled whole, but "expect to also use `#[allow]`
  attributes generously". For `restriction`, "It is **not** recommended to enable the
  whole group, but rather cherry-pick lints". `nursery` is not recommended. [S26]
- Configurable thresholds include `too-many-lines-threshold` (default 100),
  `too-many-arguments-threshold` (7), `cognitive-complexity-threshold` (25),
  `max-struct-bools` (3), `allow-unwrap-in-tests`, `allow-expect-in-tests` and
  `allow-panic-in-tests` (all false), plus `disallowed-methods`, `-types` and `-macros`.
  [S27]
- `[workspace.lints]` (Rust 1.74+), `[workspace.dependencies]` (members write
  `dep.workspace = true`, features are additive) and `[workspace.package]` centralise
  policy. [S5]
- Don't write `#![deny(warnings)]` in source, because it "opts out of Rust's famed
  stability". Use `RUSTFLAGS="-D warnings"` / `-D warnings` in CI instead. [S31]
- API guidelines for docs: C-CRATE-DOC, C-EXAMPLE, C-FAILURE (document errors and
  panics). [S4]

### 11. TypeScript and React

- `strict` enables `noImplicitAny`, `strictNullChecks`, `useUnknownInCatchVariables`
  and others. Some useful flags sit *outside* strict: `noUncheckedIndexedAccess` ("add
  `undefined` to any un-declared field"), `exactOptionalPropertyTypes`,
  `noImplicitOverride`, `noPropertyAccessFromIndexSignature`, `noImplicitReturns` and
  `verbatimModuleSyntax`. [S32]
- typescript-eslint: `recommended` holds the rules "whose reports are almost always for
  a bad practice". `strict` and `strict-type-checked` add opinionated and type-aware
  rules, and their rule sets "may change outside of major version updates", so pin the
  version. [S33]
- Rules of Hooks: call hooks only at the top level and only from components or custom
  hooks, enforced by `eslint-plugin-react-hooks`. [S34]
- "Effects are an escape hatch." Compute derived data during render, put event logic
  in handlers, reset with `key`, prefer `useSyncExternalStore` for subscriptions, and
  wrap data fetching in custom hooks with race-safe cleanup. [S35]
- At scale, pair `useReducer` with context through a provider module that exposes
  `useX()`/`useXDispatch()`. That "keeps the components clean and uncluttered". [S36]
- Accessibility: the WAI-ARIA Authoring Practices Guide gives patterns for widgets
  (roles, states, keyboard support) and fundamentals (landmarks, accessible names).
  [S37]

### 12. Dead code, duplication, complexity, file size

- **cargo-machete** "detects unused dependencies … in a fast (yet imprecise) way" on
  stable, with `[package.metadata.cargo-machete] ignored`. [S38] **cargo-udeps** needs
  nightly to run, matches crates by name, and can misreport doc-test-only dependencies.
  [S39]
- **knip** "finds and fixes unused dependencies, exports and files" in TS projects.
  [S40]
- **jscpd** detects duplicates in 220+ formats, including TS/TSX and Rust. For CI it
  suggests `--fail-on-new-clones` or a baseline over a global threshold. [S41]
- ESLint `max-lines` defaults to 300 ("Some people consider large files a code smell"),
  with `max-lines-per-function`, `complexity`, `max-depth` and `max-params` alongside.
  [S42]
- **cargo-semver-checks** lints API changes against a baseline (crates.io, or
  `--baseline-rev` for unpublished crates). [S43]

### 13. Documentation: ARCHITECTURE.md (matklad)

For 10k–200k-line projects, keep an ARCHITECTURE file next to README and CONTRIBUTING
with:

- a bird's-eye view of the problem;
- a codemap ("a map of a country, not an atlas of maps of its states");
- names, not links, for files, modules and types;
- invariants, "especially ones expressed as an absence";
- layer boundaries;
- cross-cutting concerns.

Keep it short, put only slow-changing facts in it, and revisit it "a couple of times a
year". [S2]

---

## (b) The repo today

### Measured facts (non-test code unless stated)

| Metric | Value |
|---|---|
| Rust source (tracked) | 121,845 lines in `crates/` |
| TS/TSX (tracked) | 49,066 lines in `ui/src` |
| Rust files > 1,000 lines | 17. Largest: `juggler-app/src/commands.rs` 14,386; `juggler-stats/src/generated.rs` 3,875 (generated); `juggler-supervisor/src/supervisor.rs` 2,883; `juggler-app/src/usage_stats.rs` 2,721; `link.rs` 2,382; `machine_health.rs` 2,023 |
| TS files > 500 lines | 18. Largest: `lib/bridge.ts` 4,820; `components/RightInspector.tsx` 3,036; `components/WorkflowRail.tsx` 1,988; `screens/FirstRun.tsx` 1,544; `App.tsx` 1,406 |
| Rust functions > 100 lines | 36 of 2,814 (> 200: 5). Clippy's `too_many_lines` works: 9 `allow`s |
| TS functions/components > 100 lines | 66 of 704 (> 200: 30). `App` is **1,231 lines**, with 39 `useState` and 24 `useEffect` |
| Tauri commands | ~225 registered (104 in `commands.rs`). Only **28 are `async`**, so ~197 run on the **main thread** [S6], including `environment_scan`, `environment_scan_full`, `cleanup_inventory` and `switch_execute` |
| `generate_handler!` list | 221 lines in `juggler-app/src/lib.rs`. `run()` is 344 lines |
| `AppState` | one struct, 25+ public fields, one `Mutex<Storage>` serialising all SQLite access (as flagged in `docs/review/deep/08-…`) |
| UI → IPC | 201 hand-typed `invoke<T>("name", {...})` strings in `bridge.ts`. Every UI call resolves to a registered command (renames included). **~12 registered commands have no UI caller**, among them 3 `legacy_aliases`, `scan_cancel`, `pricing_override_*`, `recommendation_for` and `validation_dogfood_record` |
| ts-rs | 114 exported types into `ui/src/types/generated/`. **No CI drift check** |
| Tool-id literals (prototype lint, test modules stripped) | **Backend outside `juggler-adapters`: 91** (`juggler-stats/generated.rs` 24, `juggler-handover/validation.rs` 21, `fidelity.rs` 20, `juggler-app/commands.rs` 11, `usage_pipeline.rs` 7, `juggler-core/recommendation.rs` 3 …). **UI: 64** (`bridge.ts` 37, mostly preview fixtures; `FirstRun.tsx` 17; `providers.ts` 4; `App.tsx` 3). This matches ADR-0019's ~114 / ~59 once test code is included |
| Duplicate display-name tables | `"claude-code": "Claude Code"` appears in both `App.tsx:159` and `lib/providers.ts:6` |
| Exact clones (8-line windows) | ~1.4k duplicated lines across the repo. Hot spots: `supervisor.rs` (within the file), `SwitchFlow.tsx`, `storage/mod.rs` ↔ `process_ledger.rs`, `NewSession.tsx` ↔ `NewProjectSheet.tsx`, `FleetScreen.tsx` ↔ `Settings.tsx`. **Copy-paste is not the main spaghetti vector. Per-tool branching and file size are.** |
| Adapters sharing code | Shared mechanics live in tool-named modules: `codex_environment.rs` imports `Layout`/`installs_from` from `claude_environment.rs`, and `codex.rs` imports helpers from `claude_code.rs`. ZCode's credit logic (`zcode_multiplier`, `zcode_is_peak`, `zcode_events`) sits in the shared `quota_ingest.rs`. `HarnessAdapter` has ~27 methods (ADR-0019 warns against a "mega-trait") |
| TS escape hatches | 0 `any` (good). 101 `as unknown as` (96 in `bridge.ts`, reading `window.__AIJ_*__` preview hooks). 40 non-null `!.`. 5 `eslint-disable react-hooks/exhaustive-deps` comments, **but there is no ESLint in the repo** |
| Logging | **No crate depends on `tracing`** (QUALITY.md §2 requires it). Diagnostics go through `eprintln!` (`juggler-app/src/lib.rs`, `usage_stats.rs`, …) |
| Lint allowances | 26 `needless_pass_by_value` (Tauri `State` convention), 34 `cast_precision_loss`, 10 `unwrap_used`, 8 `allow(clippy::pedantic)` in tests. None carries a `reason` |
| Config files | `rustfmt.toml` (minimal), `deny.toml` (good). **No `clippy.toml`. No ESLint config. No `[workspace.dependencies]`** (versions drift in spelling: `serde = "1"` vs `"1.0.229"`, `sha2 = "0.11"` vs `"0.11.0"`) |
| Docs | ADRs are strong (0000–0018, 0019 pending). **No ARCHITECTURE.md and no crate READMEs** |

### What is already good (keep)

- The workspace is flat under `crates/` with a virtual root, and the crate split has a
  real domain core (`juggler-core`, sync and pure apart from storage).
- Workspace lints: `unsafe_code = "forbid"`, `missing_docs = "deny"`, pedantic, and the
  `unwrap/expect/panic/todo/unimplemented` lints, with `-D warnings` applied in CI
  rather than in source, as [S31] recommends. One audited `unsafe` exception lives in a
  separate crate.
- thiserror enums per crate. `AppError` serialises a stable wire shape
  (`impl Serialize for AppError` → `ErrorWire`), exactly as Tauri recommends.
- DTOs (`*Info`) are separate from domain types, and ts-rs is the source of truth for
  wire types.
- Explicit adapter registration (`AdapterRegistry::register`). No `inventory`/`linkme`.
- Testing culture: proptest in core, fixtures per adapter, fuzz targets, guardrail
  suite, golden files with a bless flag, visual and a11y gates, perf budgets.
- cargo-deny and SHA-pinned actions. The capability file grants only `core:default`
  plus two dialog permissions.

### Gaps ranked by risk

| # | Gap | Risk | Why it matters |
|---|---|---|---|
| G1 | **About 197 sync commands run on the main thread**, including filesystem scans, SQLite under one mutex, and process kill ladders | **High**: user-visible freezes, which grow with data size (50 instances, 500-turn transcripts) | Tauri runs non-async commands on the main thread [S6]. One slow scan blocks the window event loop. Perf budgets (inspector < 200 ms) can't hold while the UI thread waits on I/O |
| G2 | **`commands.rs` at 14,386 lines, holding 104 commands with business logic inline**, plus a god-object `AppState` and one `Mutex<Storage>` | **High**: the main merge-conflict and coupling hub, which blocks ADR-0019 modules (E17–E19) | Commands validate, compute rollups and lock storage directly (`cost_rollups`, `pricing_override_set`). No service seam means features can't be tested or owned separately, and every feature touches one file and one lock |
| G3 | **Tool-id literals: 91 in the backend outside adapters and 64 in the UI, with no lint yet**, plus duplicated display-name tables | **High** for E16 (8+ tools) | Each new tool multiplies branches (the N× cost in ADR-0019). Handover pair matrices hold 41 literals |
| G4 | **Gates the docs claim but nothing enforces**: layering is "CI-enforced" (QUALITY §4) but no check exists; `tracing` is required but no crate uses it; `cargo-semver-checks` is promised in ADR-0002 but absent; a "scheduled CI fuzz job" is promised but absent; coverage has thresholds but the CI step is `continue-on-error` | **Medium-high** | Breaks principle 3, "The repo always tells the truth". Contributors trust gates that don't exist, and layering can regress silently |
| G5 | **No UI linter.** No ESLint at all (rules of hooks, exhaustive-deps, floating promises and a11y are unchecked, while disable comments pretend otherwise). Components are huge (`App` 1,231 lines, 30 components > 200 lines). IPC is stringly typed (201 hand-written `invoke` names and return types) | **Medium-high** | Hook-ordering bugs and stale-closure bugs pass review. The type of `invoke<T>` is an unchecked assertion: a Rust rename or a signature change compiles in TS and fails at runtime |
| G6 | **`bridge.ts` mixes the real IPC and the design-preview mocks** in one 4,820-line module, so the fixtures ship in the production bundle. 96 `as unknown as` casts sit in it | **Medium** | Bundle bloat and a confusing seam. Mocks drift from the real contract because nothing ties them to the generated types |
| G7 | **No ts-rs drift check.** Generated bindings are written by `cargo test`, and CI never diffs them | **Medium** | Rust and TS types can silently disagree on `main` |
| G8 | **Adapters share code through tool-named modules** (Codex depends on `claude_*`; ZCode logic sits in shared `quota_ingest`). `HarnessAdapter` has ~27 methods | **Medium**: grows with E16 | Adapter #3+ copies from Claude or Codex, or imports a sibling. That is the coupling ADR-0019 §2 forbids |
| G9 | **Lint allowances without reasons, lint boilerplate in tests, no `[workspace.dependencies]`, no `clippy.toml`** | **Low-medium** | Allowances can't be audited, and version drift across crates goes unnoticed |
| G10 | **No ARCHITECTURE.md and no crate READMEs** | **Low-medium** | A public repo of 170k lines with no codemap. New contributors learn layering from ADR prose |
| G11 | **Errors swallowed into defaults** in places (`cost_rollups` returns `default_cost()` on a poisoned lock) | **Low-medium** | Violates "degrade, never crash" in the wrong direction: it degrades silently with no log (and there is no tracing) |
| G12 | **Dead IPC surface**: ~12 registered commands with no UI caller, including 3 "legacy aliases" with no removal date. The PR template has its "Visual baselines" section twice | **Low** | "No dead code pretending to be alive" (QUALITY §1.3) |

---

## (c) Draft ENGINEERING GUIDE (for `docs/ENGINEERING.md` or a new §10 of QUALITY.md)

> Scope: every PR. Words: **MUST** = CI or review blocks; **SHOULD** = default, deviation
> explained in the PR. Every rule names its enforcement. "Proposed" means the check
> does not exist yet (see (e)).

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
costs N×. *Enforced:* proposed `scripts/tool_literal_lint.py` (prototype in
`research/tools/tool_literals.py`) with `scripts/tool_literals.allow`, an allowlist of
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
`scripts/ipc_audit.py` (prototyped for this report: it found 0 broken calls and ~12
uncalled commands). **SHOULD, later:** generate typed command wrappers (tauri-specta
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

## (d) Code-review checklist

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

## (e) Prioritised enforcement steps

Sizes: **S** ≤ ½ day, **M** 1–3 days, **L** a release branch.

| # | Step | Size | Closes |
|---|---|---|---|
| 1 | **Tool-literal lint** (`scripts/tool_literal_lint.py` from the prototype) with a shrink-only allowlist, in `just verify` and CI | S | G3 (stops growth now) |
| 2 | **Layering check** from `cargo metadata` against `docs/plan/layering.toml` (adds `juggler-stats` and `juggler-account`, which §4 doesn't list), plus the sibling-adapter import rule | S | G4, G8 |
| 3 | **IPC audit script**: invoke names ↔ handler list, uncalled commands, sync-command allowlist (T1), command body ≤ 40 lines, `<feature>_` prefix | S | G1 (stops growth), G12 |
| 4 | **ts-rs drift check** (`git diff --exit-code -- ui/src/types/generated` after tests) | S | G7 |
| 5 | **`clippy.toml` + workspace lint additions** (allow-reason, print, dbg, in-tests flags, `disallowed-methods`), converting today's `allow`s to `expect(reason)` | S | G9 |
| 6 | **Size budget script** (Rust non-test and TS lines) with a shrink-only allowlist | S | G2, G5 (stops growth) |
| 7 | **Truth pass on QUALITY.md and ADR-0002**: either add `cargo-semver-checks --baseline-rev` for `juggler-adapter` and a scheduled fuzz workflow, and make coverage blocking, or reword the claims. Remove the PR template duplicate | S–M | G4 |
| 8 | **ESLint flat config** (typescript-eslint strict-type-checked, react-hooks, jsx-a11y, max-lines, import boundaries) as `pnpm lint` in the `ui` job. Start with `warn` for the size rules and an override list for today's big files | M | G5 |
| 9 | **cargo-machete + knip + jscpd (`--fail-on-new-clones`)** in CI as pinned devDependencies and binaries, plus `cargo +nightly udeps` in a monthly workflow | S | G12 |
| 10 | **Adopt `tracing`**: subscriber in `juggler-app` with a redaction layer, replace `eprintln!`, add instrument spans on commands | M | G4, G11 |
| 11 | **Make the slow sync commands async** (scans, cleanup inventory, switch execute, kill ladder) with `spawn_blocking`, then shrink the allowlist from step 3 | M | G1 |
| 12 | **`[workspace.dependencies]`** for shared crates (serde, serde_json, thiserror, sha2, rusqlite, tokio, rustls, prost) | S | G9 |
| 13 | **ARCHITECTURE.md** (matklad style) and the "must not know" line in each crate doc | M | G10 |
| 14 | **ToolSpec + `tools_catalog`**: move display names, links, sign-in words and the first-run list into specs, make `FirstRun.tsx`/`providers.ts`/`App.tsx` branch-free, and add the unknown-tool render test (ADR-0019 migration step 1) | M | G3 |
| 15 | **Neutral `common/` modules in `juggler-adapters`**: move `Layout`/`installs_from` out of `claude_environment.rs`, ZCode credit logic out of `quota_ingest.rs`, and split `HarnessAdapter` into capability traits | M | G8 |
| 16 | **tsconfig extra-strict flags** (`noUncheckedIndexedAccess`, `noImplicitReturns`, `noImplicitOverride`, `verbatimModuleSyntax`) and fix the fallout | M | G5 |
| 17 | **Split `bridge.ts`** into `lib/ipc/<feature>.ts` and a lazily loaded `src/preview/`, with typed `PreviewHooks` | M–L | G6 |
| 18 | **Split `commands.rs` and `AppState`** into `features/<x>/{service,repo,commands,dto,events}` with per-feature managed state, mechanically and with no behaviour change, in one release branch (ADR-0019 migration step 3) | L | G2 |
| 19 | **Shell registry in the UI** (screens, inspector tabs, settings sections, Home cards), so that `App.tsx` drops below 400 lines and its state moves into hooks or reducer+context | L | G5 |
| 20 | **Storage concurrency ADR**: per-repository connections or a pool instead of one `Mutex<Storage>` | M | G2 |
| 21 | **Typed command wrappers** (tauri-specta once 2.0 is stable, or an in-repo generator), with an ADR | M | G5 |

Steps 1–7 and 9 are all **S**. Together they turn every "stop the bleeding" rule into a
check within a week, without touching product code. Steps 14, 15 and 18 are the
ADR-0019 migration itself.

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
