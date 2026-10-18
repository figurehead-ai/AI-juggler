# AI Juggler — Execution plan: epics, tasks & validation gates

This is the execution companion to the [spec](../SPEC.md): the spec defines *what*; this plan defines *how, in what order, and how we prove it works*.

**Sources of truth**

- **[SPEC.md](../SPEC.md)** — product & architecture requirements.
- **This document** — milestone → task breakdown, acceptance criteria, validation gates.
- **GitHub Issues** — live task tracking (one tracking issue per milestone + one issue per task, created as each milestone starts; labeled `m0`…`m8`, `area:*`, `type:task|gate`).
- **[QUALITY.md](./QUALITY.md)** — engineering standards. **Every task in this plan inherits the Definition of Done from QUALITY.md — no task is "done" without it. There are no shortcuts and no task-specific exceptions.**

**Process rules**

1. Work happens in PRs against feature branches; `main` is always green and releasable (branch protection + required CI from M0 onward).
2. Each task closes with its issue's acceptance checklist green and the DoD checklist green.
3. Each milestone closes with a **validation gate** (below) — a full validation pass, a committed report (`docs/qa/gate-MX.md`), and **user confirmation before the next milestone starts**.
4. Scope changes go through the spec first (PR), then this plan — never sideways into code.
5. Autonomous operation: unattended build runs follow the contract in [RUNBOOK.md](./RUNBOOK.md) — same PR flow, same DoD, hard stops at gates and genuine blockages.

**Milestone status**

| Milestone | Scope | Status |
|---|---|---|
| M0 | Foundation, CI, domain model, adapter trait, fake Provider, supervisor skeleton | 🟡 Ready to start |
| M1 | Claude Code accounts + parallel launcher + instance board + terminal | ⚪ Planned |
| M2 | Codex + ZCode adapters, machine health, crash recovery | ⚪ Planned |
| M3 | Usage & quota dashboards | ⚪ Planned |
| M4 | Handover engine (T1–T4) | ⚪ Planned |
| M5 | Release engineering → **v0.1.0-beta.1** | ⚪ Planned |
| M6 | Fleet link → **v0.2** | ⚪ Planned |
| M7 | Fleet control → **v0.3**; Win/Linux bring-up | ⚪ Planned |
| M8 | Stabilization → **1.0** | ⚪ Planned |

---

## Execution phases (Stage-1 resequence per D-01/D-13; task detail in docs/tasks/)

> Terminology follows [ADR-0008](../adr/0008-terminology-charter.md)
> and the [legacy→canonical glossary in SPEC §3](../SPEC.md).

The canonical task breakdown is now the **E-catalog**
(`docs/tasks/E1..E10`, 107 tasks, evidence-gated). The M-milestones
below retain their IDs for tracking continuity; their *scope text* is
re-sequenced around the settled thesis **Switch → See → Clean**, with
Fleet integrated at launch (D-13), not deferred:

- **Phase 0 — Foundation & truth** (= M0/M1 done + E8 evidence
  machinery + E9 docs you are reading): engine, Claude adapter,
  instance-centric UI, onboarding scan/import, handover v0 — plus the
  evidence store, visual test harness, and gates that make every later
  claim auditable.
- **Phase 1 — See** (E4): enriched Instance read model, indicator
  grammar, the six-tab Right Inspector, honest fields, desktop
  adapter contract + Focus Window.
- **Phase 2 — Switch** (E2, runs concurrently with Phase 1 where
  `Depends:` allows): Switch Graph, pack v2 + redaction policy,
  per-pair fidelity matrix, execution pipeline with rollback, hybrid
  door, switch-back resume+delta, validation programme (D-04).
- **Phase 3 — Clean** (E5): spawn ledger, verified ownership,
  Cleanup Centre, tiers per D-08, audit, crash recovery.
- **Phase 4 — Smart onboarding generalised** (E3): cross-Provider
  scan/inference/recommender, six flows, sign-in queue.
- **Phase 5 — Quota/cost** (E6, research-gated by AJ-E909): two-clock
  engine, forecast, cost ledger.
- **Fleet at launch** (E7, D-13): pairing/transport/identity early;
  remote Inspector/switch/cleanup under per-machine permissions;
  security review is a beta-exit gate.
- **Release engineering** (= M5): signed builds, updater, channels.
- **Workflows & beta ops** (E10): derived workflows, safety nets,
  dogfood programme, beta-exit evidence.

**Slip rule:** when a phase is late, cut widening/demoted scope
before cutting fixtures/validation. Demoted items (review-queue
product, templates library, present mode) stay deferred with explicit
revisit triggers. **Gate evidence** for every phase now includes the
switch-budget proof (AJ-E807/E808 fidelity register), cleanup
zero-false-kill proof, and the dogfood thresholds (AJ-E809).

The per-task source of truth is `docs/tasks/TASK-CATALOG.md`; the
tables below are the historical M-view, updated where scope moved.

## M0 — Foundation & CI *(epic)*

Goal: a clean, enterprise-grade workspace where everything after it can be built and continuously validated.

| ID | Task | Deliverables | Acceptance criteria |
|---|---|---|---|
| M0-1 | Cargo workspace scaffold & lint baseline | Workspace with `juggler-core`, `juggler-adapter`, `juggler-adapters`, `juggler-supervisor`, `juggler-handover`, `juggler-link`, `juggler-app` (stub); `rustfmt.toml`; workspace `[lints]` (clippy `-D warnings` + pedantic subset); `rust-toolchain.toml`; `deny.toml`; task runner (`justfile`) | Fresh clone → `just verify` (fmt + clippy + test + doc) passes with zero warnings; every crate has crate-level docs and `#![forbid(unsafe_code)]`; MSRV pinned and documented |
| M0-2 | CI pipeline | GitHub Actions: fmt, clippy, test (ubuntu), **build matrix (macOS arm64/x64, Ubuntu, Windows)**, `cargo-deny` (advisories/licenses/bans), `cargo doc`, coverage (llvm-cov, artifact upload) | All jobs green on a PR before merge (branch protection enforces); a deliberately broken PR (lint error) is rejected — verified once during setup |
| M0-3 | Domain model & SQLite storage | Types from spec §3 (`Account`, `Profile`, `SessionRecord`, `UsageSnapshot`); rusqlite (WAL) with versioned migrations; repository layer; atomic-write helpers | Round-trip property tests for every entity; migration up/down tested from empty schema; no panics on corrupt DB (degrades + logs) |
| M0-4 | Adapter trait, registry & fake harness | `HarnessAdapter` trait + `Capabilities` + `DiscoveryReport` + `LaunchSpec` + `SessionWatcher` (spec §4); `verified_against` versioning; **fake harness** binary: scriptable CLI that emits JSONL transcripts, supports resume, and can be told to hang/burn-CPU/exit for testing | Integration test: fake harness is discovered (version reported), launched, its transcript watched and parsed, and resumed — end to end |
| M0-5 | Supervisor skeleton | portable-pty spawn with per-recipe env; lifecycle event bus; process-tree tracking (ppid map); SIGTERM→SIGKILL stop ladder; output ring buffer; supervision journal for crash re-adoption | Integration tests: spawn/capture/stop cleanly; SIGKILL child → `Failed` event + journal entry; simulate app restart → children re-adopted by journal correlation |
| M0-6 | **Gate V0** | Validation report `docs/qa/gate-M0.md` | Gate checklist (see below) fully green; user confirmation recorded in the tracking issue |

**Gate V0 — Foundation.** Automated: full CI matrix green (3 OSes), coverage baseline recorded, `cargo-deny` clean. Functional: M0-3/4/5 acceptance tests demonstrated from a fresh clone. Review: first code-quality audit against [QUALITY.md](./QUALITY.md) (findings fixed before pass). Confirmation: report committed; **next milestone starts only after user sign-off.**

---

## M1 — Claude Code accounts & first UI *(epic)*

> **UI/UX checkpoints (maintainer requirement, review round 4):** M1 runs with explicit design reviews — the design direction (tokens, layout, key screens) is reviewed at M1-1 **before** screens get built out, and each major screen (M1-3 launcher, M1-4 terminal, M1-6 board) is reviewed as it lands. Work pauses at each checkpoint; the bar is a super-modern, highly usable UI that fits the juggling workflow.

Goal: the founding pain point solved — two Claude accounts in parallel, one click, zero interference.

| ID | Task | Deliverables | Acceptance criteria |
|---|---|---|---|
| M1-1 | Tauri app shell + UI scaffold | Tauri 2 app (ADR-0001); React 18 + TS + Tailwind + shadcn/ui; typed command/event bridge; locked CSP; design tokens; base components | App launches < 1.5s cold; no remote content loads (CSP test); typed events have generated TS types (single source); **design-direction checkpoint passed — maintainer reviewed tokens + layout + shell screenshots before screen build-out** |
| M1-2 | Claude adapter: discovery & account prep | Binary discovery + version (`verified_against` check with drift warning); `prepare_account` creates isolated `CLAUDE_CONFIG_DIR` (0700); login handoff (user completes `claude` login in guided terminal); login-state detection | Managed dir created with correct perms; login detected without reading token contents (presence/keychain-namespace only); version drift surfaces a warning banner |
| M1-3 | Launch settings & parallel launcher | Recipe CRUD; settings overlay (model etc. via managed `settings.json`); one-click launch with env injection; per-recipe env isolation | Two recipes of the same Provider run simultaneously without cross-talk (env + keychain isolation verified); **guardrail test: zero writes to `~/.claude`** |
| M1-4 | Embedded terminal | xterm.js + PTY streaming via Tauri events; resize; scrollback cap; copy/paste; multi-tab; "open in external terminal" | 60 fps with 20 streaming sessions in UI perf test; resize/echo correctness; terminal survives UI panel close/reopen |
| M1-5 | Session watcher & transcript parser v1 | notify-based watching of managed session dirs; JSONL parser (uuid/parentUuid/message/usage) with committed fixture corpus; resume dedupe (message uuid) | Fixture corpus (per known version shapes) parses with 100% line coverage on samples; malformed lines skip + count, never crash; dedupe across resumed sessions verified |
| M1-6 | Projects + Instances + Sessions screens (ADR-0007) | Juggling-centric navigation: left rail = Projects / Instances / Sessions; projects as first-class domain (dir → active instance + account history + launch-with); instance panel (status, account/recipe, kind Terminal/Desktop, project, actions); sessions list; global menus top-right (Overview, Accounts, Usage, Fleet) | Live updates < 300 ms after state change; launch → project appears with active instance → open panel → stop; handover-ready data model (project ↔ session ↔ account lineage) |
| M1-7 | **Gate V1** | `docs/qa/gate-M1.md` | Two real Claude accounts parallel for a full working session each; guardrail suite green; quality audit #2; user confirmation |

---

## M2 — Codex + ZCode adapters & machine health *(epic)*

| ID | Task | Deliverables | Acceptance criteria |
|---|---|---|---|
| M2-1 | Codex adapter: discovery & account prep | Binary/version discovery; isolated `CODEX_HOME`; login handoff; recipe `*.config.toml` generation (post-0.134 file semantics) | Two Codex accounts parallel; generated config validated against the current Codex config reference (test fixtures) |
| M2-2 | Codex watcher | rollout JSONL parser (fixtures: `session_meta`, `response_item`, `token_usage_record`); resume support | Session indexed from `session_meta` without full replay; token accounting matches fixture totals exactly |
| M2-3 | ZCode adapter | App-bundle + bundled CLI discovery (version-pinned); `HOME`-isolated account prep; login via `--no-browser` handoff; WAL-safe SQLite reads (`session`, `model_usage`) | ZCode sessions + usage indexed read-only; isolation verified (state stays in managed HOME, incl. credentials file) |
| M2-4 | Adopt-existing flow | Import wizard: fork an existing `~/.claude` / `~/.codex` / `~/.zcode` install into a managed account copy | Source dirs never modified (verified by fs fixtures); user confirms a diff summary first |
| M2-5 | Stuck detection v1 | Heuristics per spec §5.1 with user-tunable thresholds; statuses (Stuck?, WaitingUser, QuotaBlocked, Failed) | Fixture corpus (fake Provider scripts: hang / wait-permission / quota-exit / crash) classified with ≥ 90% precision & recall |
| M2-6 | Resource manager | sysinfo collectors (staggered); process-tree attribution; `footprint_stats` learning; admission advisor (OK/Tight/No) in the launch dialog; overload alerts; dashboard machine gauges | Advisor verdicts within ±20% of observed peaks after calibration runs; overload fixtures trigger the right alert classes |
| M2-7 | Crash recovery | Exit classification (OOM/quota/crash heuristics); recovery cards (resume/relaunch/hand over); opt-in per-recipe auto-restart (same recipe only); app-crash re-adoption | Crash fixtures → correct class + one-click resume restores the session; auto-restart never changes account (guardrail test) |
| M2-8 | **Gate V2** | `docs/qa/gate-M2.md` | Three Providers on one board; stuck + resource fixtures pass; quality audit #3; user confirmation |

---

## M3 — Usage & quota *(epic)*

| ID | Task | Deliverables | Acceptance criteria |
|---|---|---|---|
| M3-1 | Claude quota feed | Managed statusline capture → `rate_limits` events (5h/weekly, resets) | Feed matches `/usage` within tolerance on live accounts across a full window |
| M3-2 | Usage engine | Event store, window reconstruction, rollups, burn-down projections, confidence labels (Measured/Computed/Estimated) | Property tests on window math; projections within stated error bounds on fixture timelines |
| M3-3 | ZCode credit computation | Credit formula (published multipliers, off-peak windows) + plan config UI | Computed credits match vendor-reported usage within ±5% on live accounts |
| M3-4 | Codex estimation | Token-based window reconstruction + calibration loop | Estimates labeled as estimates; calibration improves over two observed windows |
| M3-5 | Dashboards, tray & alerts | uPlot timelines (per-account/session/window); CSV export; tray aggregate state; threshold notifications | 10k-point timelines render < 100 ms; alerts fire at configured thresholds in fixture-driven tests |
| M3-6 | **Gate V3** | `docs/qa/gate-M3.md` | Accuracy criteria (Claude/ZCode ±5% vs vendor-reported); quality audit #4; user confirmation |

---

## M4 — Handover engine *(epic)*

| ID | Task | Deliverables | Acceptance criteria |
|---|---|---|---|
| M4-1 | Transcript IR completion | All three parsers → neutral IR; golden-file tests per Provider × version | IR round-trips (lossless where format allows); golden diffs fail loudly on format drift |
| M4-2 | T1 resume + lineage | Native resume UI; lineage store (A→B→A chains) | Resume from board in ≤ 2 clicks; lineage rendered correctly after multi-hop fixture chains |
| M4-3 | T2 session transplant | Claude (transcript copy + fork-on-collision) and Codex (rollout copy) cross-account transplant | Transplanted session resumes on target account with full history (fixtures + live); re-transplant collision handled; source untouched |
| M4-4 | T3 transcript replay | Full-conversation rendering → target priming per Provider; fidelity labeling in UI | Cross-Provider switch carries complete conversation content (verified by fixture assertions on the primed context) |
| M4-5 | T4 packs | Generate / preview / edit / export / import | Pack import creates a primed session in any managed Provider; format versioned |
| M4-6 | Project context sync | Rules-file conversion (AGENTS.md ↔ CLAUDE.md ↔ `.zcode/AGENTS.md`), memory/todo sync; diff preview; apply on click | Conversion golden tests; zero un-previewed writes (guardrail test) |
| M4-7 | Handover wizard + **Gate V4** | Wizard (tier + quota headroom per target); `docs/qa/gate-M4.md`; per-pair validation gate: `docs/qa/gate-E2-validation.md` (AJ-E210, regenerate with `just e2-gate`) | Dogfood criterion: A→B→A round trip preserves continuity; quality audit #5; user confirmation |

---

## M5 — Release engineering *(epic → v0.1.0-beta.1)*

| ID | Task | Deliverables | Acceptance criteria |
|---|---|---|---|
| M5-1 | Self-update | Updater keys + `latest.json` feed via tauri-action; channels (beta); settings UI; disable switch | Signature verification cannot be bypassed (negative test with bad sig); channel switch works |
| M5-2 | Signing & notarization CI | Developer ID signing + `notarytool` via Vuxtra's Apple Developer account; GitHub Environments secrets; Universal macOS build | Fresh-VM install: Gatekeeper clean, no warnings; notarization log archived per release |
| M5-3 | Provider update tracking | Version recording; daily channel checks; verified-range status UI | Correct "update available" for all three Providers in fixture tests; never auto-updates a Provider (guardrail) |
| M5-4 | Onboarding polish | First-run wizard; guided account add; recipe templates | New user (fresh machine) → two parallel accounts < 5 min (scripted dry-run + human trial) |
| M5-5 | Fleet transport spike | tonic + rustls + mDNS prototype between two local instances; ADR update fixing wire format | Spike report + ADR-0004 updated; pairing + one telemetry stream demoed locally |
| M5-6 | **Gate V5 = v0.1.0-beta.1** | Tagged, signed, notarized release; README quickstart; `docs/qa/gate-M5.md` | End-to-end: install beta → auto-update to a staged newer build succeeds; quality audit #6; user confirmation → public beta |

---

## M6 — Fleet link *(epic → v0.2)*

| ID | Task | Deliverables | Acceptance criteria |
|---|---|---|---|
| M6-1 | Identity & trust store | Ed25519 identity per instance; cert derivation; paired-peer registry (`fleet/` storage) | Keys never leave the machine unencrypted; registry tamper detected |
| M6-2 | Discovery | mDNS/Bonjour advertising + browsing (`mdns-sd`) | Two instances discover each other on a LAN fixture (loopback + CI network namespace) |
| M6-3 | Pairing & permissions | 6-digit code / QR wizard; per-machine permissions (view-only etc.) | Unpaired connections dropped (negative tests); pairing < 60 s in the acceptance script |
| M6-4 | Telemetry & remote board | gRPC/mTLS streaming of heartbeats, resources, session deltas; read-only remote board | Board reflects remote state < 1 s lag under fixture load; mixed-version degradation works |
| M6-5 | Fleet alerts & **Gate V6** | Alert routing to consoles; `docs/qa/gate-M6.md`; tag v0.2.0-beta | Two machines, one board, < 60 s pairing; credentials-never-leave-machine guardrail extended to link tests; quality audit #7; user confirmation |

## M7 — Fleet control *(epic → v0.3)*

| ID | Task | Deliverables | Acceptance criteria |
|---|---|---|---|
| M7-1 | Authenticated commands & audit log | Console→node command auth; per-command audit trail | All commands attributable; replay of captured traffic rejected |
| M7-2 | Remote lifecycle | Remote launch/stop/attach from console (click-executed) | Full manage workflow across 3 machines in acceptance script |
| M7-3 | Cross-machine handover | Pack fetch on demand; T2/T3 across machines with path & binary checks surfaced honestly | Cross-machine handover demo; T2 constraints pre-validated and explained in-wizard |
| M7-4 | Aggregated usage | Fleet-wide usage views & alerts | Cross-machine rollups correct under clock-skew fixtures |
| M7-5 | Windows/Linux bring-up begins | CI-only ports of core+adapters (no packaging yet) | `just verify` green on Win/Linux for core crates |
| M7-6 | **Gate V7 = v0.3.0-beta** | `docs/qa/gate-M7.md` | Acceptance script across 3 machines; quality audit #8; user confirmation |

## M8 — Stabilization *(epic → 1.0)*

| ID | Task | Deliverables | Acceptance criteria |
|---|---|---|---|
| M8-1 | Beta feedback hardening | Triage-driven fixes; perf & memory pass | No P0/P1 open; perf budgets (spec §1) met on reference hardware |
| M8-2 | Security review | Internal security review vs SECURITY.md commitments; dependency audit | Findings fixed or documented with mitigations |
| M8-3 | Docs completeness | User guide, adapter guide (public API), troubleshooting | A new contributor can add a fake adapter from docs alone |
| M8-4 | **Gate V8 = 1.0** | Stable channel opens; `docs/qa/gate-M8.md` | Full regression matrix; quality audit #9; user confirmation → **1.0.0** |

---

## Validation gate definition (applies to every `Gate VX`)

1. **Automated:** full CI matrix green (macOS arm64/x64, Ubuntu, Windows) · guardrail test suite green · coverage ≥ targets (QUALITY.md §5) · `cargo-deny` clean · benches within stated budgets (no unexplained regressions).
2. **Functional:** every acceptance criterion of the milestone's tasks executed and evidenced (command output, screenshots, or fixture reports) — no criterion waived.
3. **Quality audit:** a pass over the milestone's diff against [QUALITY.md](./QUALITY.md) (structure, naming, docs, tests, error handling, architecture conformance); findings are fixed *before* the gate passes, not deferred.
4. **Report:** everything above compiled into `docs/qa/gate-MX.md` (committed artifact) and linked from the milestone tracking issue.
5. **Confirmation:** the next milestone does not start until the user signs off on the report.
