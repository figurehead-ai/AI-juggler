# AI Juggler — Engineering quality standards

This is an enterprise-grade, open-source product. The code *is* the product: thousands of developers will read it, and everything here exists so that reading it is a pleasure and auditing it is easy. **These standards apply to every task in [EPICS.md](./EPICS.md) without exception — "Definition of Done" below is part of every single requirement.**

## 1. Principles

1. **Correctness first, then clarity, then performance** — in that order. Performance work is driven by benches, never guesses.
2. **No shortcuts, no quick hacks.** A hack that saves an hour costs every future reader. If a proper fix is out of scope for a task, the shortcut is documented as an issue *and* an ADR-level note, and has an owner and a milestone.
3. **The repo always tells the truth.** `main` is green and releasable; docs match behavior; CHANGELOG matches releases; no dead code pretending to be alive.
4. **Degrade, never crash.** We parse external, version-drifting formats for a living — malformed input is expected input.
5. **Boring, idiomatic Rust.** We follow the same patterns the ecosystem uses; novelty requires an ADR.

## 2. Definition of Done — every task, every time

A task is done only when **all** of the following are true:

- [ ] Acceptance criteria from EPICS.md (mirrored in the issue) demonstrably pass.
- [ ] `just verify` clean: `cargo fmt --check`, `clippy -D warnings` (workspace lints), `cargo test`, `cargo doc` — zero warnings.
- [ ] Public items documented (`#![deny(missing_docs)]` on all crates); doc comments explain *why*, not *what*.
- [ ] Errors: typed (`thiserror`) in libraries; `anyhow` only at the binary/FFI edge; **no `unwrap`/`expect`/`panic!` in production paths** (tests and truly-infallible cases with a comment excepted).
- [ ] Logging via `tracing` (structured, level-appropriate); no `println!`/`eprintln!` in library code.
- [ ] Tests: unit tests for new logic; integration/fixture tests for new flows; parser changes extend the fixture corpus.
- [ ] Architecture conformance: dependency direction rules (§4) respected; no layer violations "just this once".
- [ ] Security: no new `unsafe`; `cargo-deny` clean; no secrets/log redaction regressions (guardrail tests still green).
- [ ] Docs updated: CHANGELOG (Unreleased), relevant spec section marked implemented, ADR if a decision was made.
- [ ] PR: small and reviewable (guideline ≤ ~400 lines of diff), linked issue, checklist completed, self-reviewed diff before requesting review.

## 3. Rust standards

- **Formatting/linting:** repo `rustfmt.toml` (hard-wrapped config, no debate in PRs); workspace `[workspace.lints]` enables a pedantic subset (`clippy::pedantic` base with explicit allowances reviewed by ADR, plus `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented` denied in library crates).
- **Crate hygiene:** every crate has a crate-level doc comment with its role and a `README.md` when it has non-obvious usage. Modules stay cohesive — a module doing two jobs is two modules.
- **Naming:** type names and APIs say what they mean (`SessionRecord`, not `SessRec`); no abbreviations except established domain terms (PTY, MCP, IR).
- **Errors:** every fallible boundary has a typed error with context (`TranscriptParseError::MalformedLine { line, source }`); error types are part of the public API and semver-protected.
- **Dependencies:** minimal, audited, and justified — new top-level dependencies require a PR note with the alternative analysis. `cargo-deny` bans duplicate major versions and GPL-only entries (MIT/Apache/BSD/ISC/Unicode preferred).
- **Concurrency:** tokio runtime in the app; no async in `juggler-core` (sync, pure, testable); adapters bridge via explicit boundaries.
- **MSRV:** pinned in `rust-toolchain.toml` + documented; raised only in minor releases with a CHANGELOG entry.

## 4. Architecture rules (CI-enforced)

Allowed dependency direction — violations fail CI:

```
ui (webview) → juggler-app → { juggler-core, juggler-adapter, juggler-adapters,
                               juggler-supervisor, juggler-handover, juggler-link }
juggler-adapters → juggler-adapter, juggler-core        (never the reverse)
juggler-supervisor / juggler-handover / juggler-link → juggler-core, juggler-adapter
juggler-core → (external crates only; knows nothing about app/UI)
```

- `juggler-core` contains zero I/O beyond its storage layer; it is pure domain logic and stays fully unit-testable.
- Adapters are the **only** code that knows harness specifics (spec §2 layering rule) — any harness path leaking into core/supervisor is a defect.
- UI is a view: it holds no business logic; all state originates in Rust and arrives as pre-aggregated, typed events.

## 5. Testing standards

| Layer | Requirement |
|---|---|
| Unit (`juggler-core`, math, windows, credit formula) | Property-based tests (`proptest`) where domains are numeric; plain unit tests elsewhere |
| Parsers (transcripts, configs) | **Fixture corpus** committed under each adapter's `tests/fixtures/` (anonymized, per known version shape); golden-file assertions; malformed-input tests mandatory |
| Fuzzing | `cargo-fuzz` targets for every external-format parser; corpus committed; scheduled CI fuzz job; crashes are release blockers |
| Integration | Fake-harness-driven lifecycle tests (launch → watch → stuck → crash → recover) in CI on all three OSes |
| Benchmarks | `criterion` benches for transcript parsing, usage rollups, board event fan-out; regression beyond 10% vs recorded baseline fails CI |
| Coverage | `llvm-cov`: ≥ 85% line coverage on `juggler-core`, `juggler-adapter`, `juggler-adapters`, `juggler-handover`; ≥ 80% on `juggler-supervisor`, `juggler-link`; reported at every gate |
| UI | Playwright smoke per milestone (critical flows); component tests for board/wizard |
| Visual regression (AJ-E803) | The deterministic matrix (`ui/tests/visual/regression.spec.ts`: rail, inspector and main screens; both themes, both densities, 1280×800 and 1024×680) is compared with approved baselines rendered in the pinned Playwright container, the same image locally (`just visual-gate`) and in CI's `visual` job. Thresholds: ≤ 0.1 % differing pixels per screen; 0 pixels for text-bearing chrome (rail tabs, buttons, badges). A status shown by colour must also be in words or an accessible name (WCAG 1.4.1). A failure uploads the baseline/current/diff triptych and is linked from the PR, never dismissed in chat. Baseline changes need a "## Visual baselines" note per screen naming the task (`scripts/check_baseline_notes.py`) and maintainer approval. Raising a threshold is a quality-gate change that needs the maintainer's recorded approval |
| Guardrails | The dedicated suite (writes confinement, no network outside allowlist, no account mutation without user command, automation policy) runs in CI on every PR and at every gate |

### Performance budgets (AJ-E805)

Measured on the reference Mac by `just load-gate` at the promised scale
(10 Accounts, 50 Instances, 20 Projects, 500-turn transcripts). The
gate quotes the measured values from
`docs/qa/evidence/AJ-E805/manifest.md`, never these targets. A median
more than 10 % slower than `ui/tests/visual/load-baseline.json` (5 ms
floor) fails unless the baseline lists a waiver with a task ID and an
expiry date. `just polling-audit` (part of `just verify`) fails any
sub-second timer or sleep in shipped code without a `poll:` note.

| Budget | Target |
|---|---|
| Cold start to interactive | < 2.5 s |
| Rail render, 50 rows | < 300 ms |
| Inspector selection to paint | < 200 ms |
| Terminal input echo (local) | < 50 ms |
| Switch pack assembly, 500 turns | < 5 s |
| Orphan detection after the grace window | < 10 s |
| Idle CPU, 10 live Instances | < 2 % |
| Memory, excluding child processes | < 600 MB |

## 6. Security & supply chain

- `#![forbid(unsafe_code)]` everywhere unless an ADR approves a specific exception.
- `cargo-deny` (advisories + licenses + bans) and Dependabot on every PR; advisories are patched within a week or risk-assessed in writing.
- Secrets: never in the repo, never in logs — the redaction helper is the only sanctioned way to include paths/ids in diagnostics.
- Updater signatures and OS signing/notarization are validated at every release gate (M5+), with a deliberate negative test (tampered artifact must be rejected).
- Threat-model note maintained per release in `docs/qa/`.

## 7. Repo hygiene & process

- **Commits:** Conventional Commits; imperative subject ≤ 72 chars; body explains *why*.
- **PRs:** one concern each; the template's DoD checklist must be completed; self-review is mandatory before requesting review; review is approval-blocking for everyone (including maintainers).
- **CHANGELOG:** every user-visible change lands in `[Unreleased]` in the same PR.
- **ADRs:** any non-obvious decision gets an ADR (`docs/adr/`) — short, dated, with alternatives considered. Code links its ADR where the "why" isn't obvious.
- **Docs:** README quickstart must always work on the latest release; architecture diagrams updated in the same PR as the change.
- **Only final artifacts are committed** (see CONTRIBUTING) — clean code and docs, not exploration or conversation residue.

## 8. Validation gates (milestone-level)

Defined in [EPICS.md](./EPICS.md) (automated + functional + quality audit + report + user confirmation). Gate packets are generated, not hand-written (AJ-E807): `scripts/gate_packet.py build docs/qa/gates/<gate>.json` lists each criterion with its evidence links and verdict. A criterion is PASS only when every evidence path resolves and its tasks are DONE-EVIDENCED; otherwise it is DEFERRED with the open task IDs, or FAIL when tasks claim done without proof. The packet also carries the gallery delta, the axe summary, the measured performance table and the unresolved items. `just evidence` link-checks every `docs/qa/gate-*.md` and rejects a PASS without evidence. A gate is signed in its Sign-off section with decision, reviewer, date and commit; the beta update channel waits for a signed beta gate. The gate report template:

```markdown
# Gate VX — <Milestone>
Date: YYYY-MM-DD · Commit: <sha> · Report: docs/qa/gate-MX.md

## 1. Automated
- CI matrix: <green/failures> (links to runs)
- Coverage: <table vs targets>
- cargo-deny: <clean> · Guardrails: <pass> · Fuzz since last gate: <no new crashes>
- Benchmarks: <within budgets / regressions explained>

## 2. Functional (every acceptance criterion, with evidence)
- <Task ID>: <criterion> — <evidence link/summary> — PASS

## 3. Quality audit (vs QUALITY.md)
- Findings: <list> — Fixed in <commits> / waived with reason <never for correctness>

## 4. Known issues carried forward
- <issue links, with milestone owner>

## 5. Confirmation
- Maintainer sign-off: the maintainer — <date>
```

## 9. Reviewer checklist (mirrored in the PR template)

Structure & naming · layering conformance · error handling · test quality (not just presence) · docs on public APIs · CHANGELOG · fixture corpus grown if parsers touched · no secrets/unsafe · bench impact for hot paths · ADR if a decision was made.


## Gate vocabulary (AJ-E806 — one green/red definition everywhere)

| Gate name | Command | Runs on |
|---|---|---|
| verify | `just verify` (fmt, clippy -D warnings, tests, docs, **evidence**) | every PR |
| ui | `pnpm --dir ui build` (tsc + vite) | every PR |
| visual | `pnpm --dir ui visual:compare` (AJ-E803, thresholds + triptychs) | UI-changing PRs |
| a11y | `pnpm --dir ui a11y` (AJ-E804, axe 0 critical, both themes) | UI-changing PRs |
| perf | AJ-E805 measured table | milestones |
| evidence | `./scripts/evidence_check.sh` | inside verify |

Rules: a gate is never weakened, skipped, or re-thresholded without a
PR stating reason + task ID + maintainer approval, recorded as a
quality decision in the evidence manifest. Flaky gates are
quarantined with a task ID and a 14-day expiry — never retried
silently. visual/a11y attach when they exist (AJ-E803/E804); until
then their absence is a recorded DEFERRED, not a silent pass.
