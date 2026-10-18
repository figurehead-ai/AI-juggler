# Evidence manifest — AJ-E304

Task: `docs/tasks/E3-smart-onboarding.md` AJ-E304.
Branch: `stage3/e3-e304-recommendation`. 2026-10-17, reviewer zcode-primary.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Engine proposes every active candidate as its own Account, default ON, each with a one-line rationale | Req.1 | `juggler_core::recommendation::recommend` — pure, UI-free (E4/E5 reuse): active signed-in → "Signed in — import keeps history" (default ON), active signed-out → "Active — you'll sign in after import" (ON); the §5 worked-example fixture asserts 3 imports + 1 add, source paths riding through for execution | PASS | 2026-10-17 | zcode-primary |
| Dormant candidates default OFF under a "Found but quiet" group with one-tap include | Req.2 | Engine: dormant → default OFF, rationale "Dormant — include when you want it" (test `dormant_defaults_off`); UI: the ProposalBlock renders dormant rows under a `proposal-quiet` group headed "Found but quiet — dormant setups stay unimported until you tick them" — a checkbox each = one tap (gallery customise captures show it; Playwright asserts the group + unchecked state) | PASS | 2026-10-17 | zcode-primary |
| Installed-but-unconfigured providers proposed as "Add <P> (installed, not set up)"; NEVER created before accept | Req.3 | App `recommendation_for_impl` derives gaps from `providers_discover` (installed && !configured) → "Add Codex (installed, not set up)" items with `action: "add"`, path null (engine test pins adds carry no dir); creation happens only inside `acceptTheSetup()`'s click handler | PASS | 2026-10-17 | zcode-primary |
| Labels come from inference and are collision-suffixed in the engine, never a post-typing validation error | Req.4 | `recommend` suffixes via a BTreeMap claim: Work, Work 2, Work 3 (test `collisions_suffixed_in_engine`); the UI has no duplicate-label validation anywhere — the picker never sees an error | PASS | 2026-10-17 | zcode-primary |
| Most-recent project proposed as "Start here" on the proposal AND Summary screens | Req.5 | `start_here` = S9 seed (first of the scan's seeds); rendered "Start here: <seed> — your most recent project." inside the ProposalBlock and on the set Summary (vitest asserts `-users-dev-api` on the summary; Playwright asserts both) | PASS | 2026-10-17 | zcode-primary |
| Accept executes imports sequentially with per-item progress; a failed item degrades ITSELF only, never aborts the set | Req.6 | `acceptTheSetup()` — sequential loop, `accept-progress` list updates per item (aria-live); per-item try/catch; vitest: item 2 of 4 rejects with a coded error → 2 done + 1 failed with its `user_message` rendered + remaining items still run; Playwright partial-failure fixture: item 2 of 4 fails → exactly 3 done, 1 failed-with-reason, all 4 picked items present in the summary = 0 aborts | PASS | 2026-10-17 | zcode-primary |
| Staging only: no Account creation, login, or file copy before the accept click (SPEC §26) | Req.7 | Guardrail test `recommendation_stages_without_writing`: after the proposal, storage accounts remain EMPTY; the command is a pure read over the cached scan report; vitest staging test: the proposal alone makes zero `accountImport` calls | PASS | 2026-10-17 | zcode-primary |
| Every recommendation explainable in one line; unexplained ones don't ship | Req.8 | Each item carries exactly one rationale line, rendered next to its label (and inside the checkbox aria-label for screen readers); no rationale-less item type exists in the struct (Rust types enforce it — no Default, no partial construction) | PASS | 2026-10-17 | zcode-primary |
| Deterministic engine; property test no duplicate label / no managed candidate; zero storage writes before accept | quality gates | `deterministic_byte_identical` (serde bytes equal); `no_duplicates_no_managed_ever` over 64 adversarial rounds; app guardrail test; engine 6/6 | PASS | 2026-10-17 | zcode-primary |
| Proposal computation ≤ 100 ms for ≤ 200 candidates | perf budget | `two_hundred_candidates_within_budget`: 200 candidates + 2 gaps → 202 items in < 100 ms (measured in-test; typically sub-millisecond — pure sorting/mapping) | PASS | 2026-10-17 | zcode-primary |
| Partial-failure threshold: item 2 of 4 fails → 3 imported, 1 failed-with-reason, 0 aborts | evidence proof | Vitest (coded `io` error on the Personal import → 2 done + 1 failed + Side still imported, start-here rendered) AND Playwright `__AIJ_IMPORT_FAIL_PATHS__` fixture asserting exactly [data-state=done]×3, [failed]×1 containing "Disk full", set-summary-item count 4 — across 1280×800 + 390×844 × light+dark | PASS | 2026-10-17 | zcode-primary |
| Visual: proposal, customise-expanded (with the quiet group), accept-in-progress; 2 viewports × 2 themes; a11y readable as a single list, axe 0 | visual testing | Gallery `36-recommend` = 16 captures (teaser+customise / accept-in-progress frozen via the hanging add / partial-failure set-summary); a11y 2 themes: `proposal-item` list semantics, descriptive aria-labels ("Work: Signed in — import keeps history"), accept button name "Accept setup → (n)", axe 0 after ONE real contrast fix (⌘↩ hint faint→muted, 3.25:1 → AA); the light-theme first-scan color-contrast flash is the recorded AJ-E806 load-race class — confirmed clean via the one-re-scan protocol; full Playwright 450/450 | PASS | 2026-10-17 | zcode-primary |

## Test counts

- `cargo test -p juggler-core recommendation`: 6/6 (worked example,
  dormant-off, engine collisions, 64-round property, byte-identical
  determinism, 200-candidate budget).
- `cargo test -p juggler-app --test onboarding`: 8/8 (incl.
  `recommendation_stages_without_writing` guardrail). The
  recommendation wire type is hand-declared in `bridge.ts`
  (`RecommendationData`) matching the serde snake_case field names —
  the engine stays serde-only by design (pure, no ts-rs coupling).
- vitest: 263/263 (wizard suite 14/14: +3 E304 — summary/rationales/
  quiet-off, sequential accept with per-item degrade, staging
  no-call-before-click).
- Playwright: 450/450 (gallery 36-recommend +12; a11y +2).
- `just verify` green; workspace clippy 0.

## AJ-E806 flake record (2026-10-17)

- First `just verify` run failed `juggler-supervisor reconciler ::
  fifty_instance_soak_and_construction_regression` — p95 tick 107 ms
  vs the 100 ms budget, with machine loadavg 18.3–23.6 (the run
  overlapped this task's Playwright/vitest suites; the supervisor
  soak is wall-clock sensitive and E304 touches nothing in that
  crate). Standalone re-runs: 3/3 green (5.2–5.4 s each, p95 inside
  budget). Per AJ-E806: recorded, verified 3/3 in isolation; the
  clean re-run of `just verify` is the local arbiter and CI the
  final one.

## Notes for the next agent

- The engine is pure (`juggler_core::recommendation`) and takes
  `(candidates, gaps, start_here)` — E4/E5 surfaces can reuse it
  verbatim; the app command `recommendation_for` shows the wiring
  pattern (scan report → candidates + gaps).
- `⌘↩` accepts from anywhere in the proposal (keydown effect);
  Customise toggles expand/collapse without discarding edits.
- Zero-found collapse is AJ-E305's flow state (its Req.2), not E304's.
