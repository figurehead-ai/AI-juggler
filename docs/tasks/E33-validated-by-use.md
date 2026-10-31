# E33 — 0.4 validated by use: every combination, every real use case

> Source: the maintainer, 2026-10-30, approving the 0.4 designs (D-62):
> "it will be very important to validate and test every combination of
> those options … not just in automated way, but from user workflow
> perspective. Real example use cases of how user would use it, this
> needs to be part of your plan for 0.4."
>
> The process is [`docs/qa/0.4-validation-plan.md`](../qa/0.4-validation-plan.md).
> This epic builds its tools and runs it. Every build task in E30, E31
> and E32 names its scenarios and isn't done until they pass; the
> scenarios are walked again, together, on the release candidate.
>
> **Order.** AJ-E3301 comes first, since the build tasks need its
> reference model and scenario files from the start. AJ-E3302 to
> AJ-E3305 run alongside their epics. AJ-E3306 is the last thing before
> 0.4 ships.

### AJ-E3301 The validation kit: scenario files, the reference model, the pairwise generator, timing and the walk sheet | Status: TODO | Priority P0 | Effort L | Depends: —
**Goal**: One scenario drives the exhaustive tests, the oracle check, the scripted journey and the hand walk, so they can't drift apart.
**User story**: As the person walking P1, the sheet I fill in has the same steps and expectations the automation checked.
**Requirements**:
1. Scenario files (`docs/qa/scenarios/*.toml`) SHALL state the setup and each step's expected outcome.
2. `scripts/qa/pool_oracle.py` SHALL be written from D-62 and the research, not the Rust, with a `--selftest`; it emits L1's expected tables and checks L2's harness output.
3. `scripts/qa/pairwise.py` SHALL make deterministic all-pairs matrices with a coverage report; the matrices are checked in.
4. Click-to-feedback and click-to-done SHALL be recorded per step (0.3.2's IPC timing plus performance marks).
5. A walk sheet template and a lab setup script SHALL build each scenario's state in the throwaway HOME and tear it down (sessions stopped, processes reaped, no stray windows).
6. The replica SHALL gain pools, Watched sessions, several Apps per Account and a closed Claude app profile.
**UI/UX reasoning**: n/a.
**Performance budget**: L1's tables run in < 2 s.
**Quality gates**: the oracle's selftest in `just verify`; never pointed at a real HOME for writes.
**Evidence/validation proof**: the kit's own tests; one scenario (P1) run through all four layers as the proof.
**Visual testing**: n/a.
**Files to touch**: `docs/qa/scenarios/` (new), `scripts/qa/pool_oracle.py`, `scripts/qa/pairwise.py` (new), `crates/juggler-app/tests/qa_real_readonly.rs`, `docs/qa/0.4-walks/` (new).
**Research basis**: `scripts/qa/real_truth.py`, `replica_fixture.py`, AJ-E1529.
**Handover note**: —

### AJ-E3302 Account and App validated | Status: TODO | Priority P0 | Effort M | Depends: AJ-E3301, AJ-E3207
**Goal**: A1–A10 and N1–N5 pass by hand and in their layers.
**User story**: As the maintainer, I read a sheet per scenario with what was seen, how fast, and its score.
**Requirements**:
1. L1 grouping and migration tables; L2 on the replica; L3 pairs; L4 walks of A1–A10 and N1–N5 in the Dev edition app.
2. Every finding fixed and its scenario re-walked from the start.
**UI/UX reasoning**: test by hand, scored.
**Performance budget**: as each task states.
**Quality gates**: nothing under 4.
**Evidence/validation proof**: `docs/qa/0.4-walks/A*.md`, `N*.md`.
**Visual testing**: captures in each sheet, both themes.
**Files to touch**: `docs/qa/0.4-walks/`.
**Research basis**: the validation plan §5.
**Handover note**: —

### AJ-E3303 Links validated | Status: TODO | Priority P0 | Effort M | Depends: AJ-E3301, AJ-E3105
**Goal**: K1–K4 pass, with every count equal to the reference.
**User story**: As someone walking the chain from Home to a pool and back, every hop is one click and every count is right.
**Requirements**:
1. L2 counts on the replica for every page and tab; L3 all pairs; L4 walks of K1–K4.
2. 1 000+ ended sessions page without a stall; "N new" never jumps the list.
**UI/UX reasoning**: everything links.
**Performance budget**: ≤ 150 ms per hop from cache.
**Quality gates**: nothing under 4.
**Evidence/validation proof**: `docs/qa/0.4-walks/K*.md`.
**Visual testing**: in each sheet.
**Files to touch**: `docs/qa/0.4-walks/`.
**Research basis**: the validation plan §5.
**Handover note**: —

### AJ-E3304 Pools validated: the exhaustive tables, the pairs, and P1–P11 by hand | Status: TODO | Priority P0 | Effort L | Depends: AJ-E3301, AJ-E3009
**Goal**: Every pool rule is right in every combination, and every template's real use works end to end with real tools.
**User story**: As someone setting up a writer and a reviewer, a feature team or a sealed client project, it does what the wizard said, with real Claude Code and Codex sessions.
**Requirements**:
1. L1: membership (28 224), talk (9 216), Claude's channel (27) and migration tables agree with the reference model.
2. L2 on the replica; L3 wizard (~60 rows) and canvas (~40 rows) pairs plus the named risky triples.
3. L4: P1–P11 in the lab with real sessions messaging, holding, refusing and switching.
**UI/UX reasoning**: pools from real use (D-61).
**Performance budget**: as AJ-E3004 and AJ-E3008 state.
**Quality gates**: nothing under 4; a lab "no" from AJ-E3002 shows as not enforced, never faked.
**Evidence/validation proof**: `docs/qa/0.4-walks/P*.md`; the coverage report.
**Visual testing**: in each sheet.
**Files to touch**: `docs/qa/0.4-walks/`, `docs/qa/scenarios/`.
**Research basis**: the validation plan §3 and §5.
**Handover note**: —

### AJ-E3305 Every 0.4 screen scored against the 14 principles | Status: TODO | Priority P1 | Effort M | Depends: AJ-E3301
**Goal**: Each new or changed screen is checked for design quality, not only that it's unchanged and accessible.
**User story**: As the maintainer, the screens I open are readable, with the common choice prominent and the rare folded.
**Requirements**:
1. A checklist per screen of the 14 principles, filled in with captures at 1024 and 1280 in both themes.
2. A critique beside each gallery family (D-57); every "no" is fixed or explained.
**UI/UX reasoning**: D-62 decision 22.
**Performance budget**: n/a.
**Quality gates**: no unchecked principle on a shipped screen.
**Evidence/validation proof**: `docs/qa/0.4-walks/design.md`.
**Visual testing**: the gallery.
**Files to touch**: `docs/qa/0.4-walks/design.md`.
**Research basis**: `docs/design/PRINCIPLES.md`.
**Handover note**: —

### AJ-E3306 The release candidate walked whole: every scenario again, a day in the life, the load | Status: TODO | Priority P0 | Effort L | Depends: AJ-E3302, AJ-E3303, AJ-E3304, AJ-E3305
**Goal**: 0.4 ships only after everything passes together on the built app.
**User story**: As the maintainer starting 0.4, the things I'd do on day one already worked for someone else.
**Requirements**:
1. Every scenario walked again on the release-candidate build (not `tauri dev`), in one run.
2. X1 (a day: holds, a switch, notices, mutes, a cache clear, a crash and a restart) and X2 (a 50-member pool, 1 000+ ended sessions).
3. A summary for the maintainer: what passed, what was fixed, what's known and why.
**UI/UX reasoning**: everything flows together.
**Performance budget**: every budget in E30–E32 held on the built app.
**Quality gates**: nothing under 4; the full preflight.
**Evidence/validation proof**: `docs/qa/0.4-walks/release-candidate.md`.
**Visual testing**: captures opened by eye before commit.
**Files to touch**: `docs/qa/0.4-walks/`.
**Research basis**: the validation plan §7–§8.
**Handover note**: The maintainer's findings after release go to 0.4.x.
