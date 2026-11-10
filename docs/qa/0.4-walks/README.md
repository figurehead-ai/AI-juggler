# 0.4 walks

One sheet per scenario of the [0.4 validation plan](../0.4-validation-plan.md)
(§5), walked by hand in AI Juggler Dev on a lab, as AJ-E3302 to AJ-E3306
require. The sheets are made from the scenario files, so a walk and its
automation can't drift apart.

## Walk one

1. Set up its lab — a throwaway HOME shaped like a heavy real Mac, with the
   scenario's projects, Apps, sessions and pools seeded into AI Juggler
   Dev's data folder there. Its sheet is the tracked `<ID>.md` here, made
   by `scripts/qa/scenario.py sheet ID --out …` when the scenario is added
   (`scenario.py check` fails without it; setup never writes into the repo):

   ```bash
   scripts/qa/lab_scenario.sh setup P3
   ```

2. Open AI Juggler Dev on it (its own HOME, so nothing real is read or
   touched), with each step timed:

   ```bash
   scripts/qa/lab_scenario.sh open P3
   ```

   In the window, set `localStorage["aij.walk-timing"] = "1"` once (or open a
   Dev window with `?walk`): every click or ↩ then logs
   `[walk] feedback 42 ms · done 180 ms`, and `window.__AIJ_WALK__` keeps
   the steps for the sheet's ms column.

3. Walk the sheet's steps in order. For each: what showed, the two times,
   and a score of 1–5 for works, speed, clarity and layout; captures in
   both themes at 1024 and 1280, each opened and checked by eye before
   it's committed.

4. Tear it down — the lab's processes stopped, the folder removed:

   ```bash
   scripts/qa/lab_scenario.sh teardown P3
   ```

## Pass

Every step does what it promised end to end, nothing is scored under 4,
feedback is ≤ 100 ms, and nothing is cut off or overlaps. A failure is a
finding on its task; once fixed, the whole scenario is walked again from
the start, not just the step that failed.

## What a scenario needs

`scripts/qa/scenario.py list` shows each scenario's needs:

- **replica** / **lab** / **mock** — runs on the lab alone;
- **live-claude** / **live-codex** — real sessions of that tool, on the lab
  accounts only (`docs/qa/runtime-lab.md`); never anyone's real account;
- **rc** — the release-candidate build, for AJ-E3306.

## The kit

| Piece | Where |
|---|---|
| Scenario files | `docs/qa/scenarios/*.toml` (`scripts/qa/scenario.py check`) |
| The pools reference model | `scripts/qa/pool_oracle.py` (`--selftest`) |
| All-pairs arrays, with coverage | `docs/qa/pairwise/` (`scripts/qa/pairwise.py check`) |
| The lab | `scripts/qa/lab_scenario.sh`, seeding through `crates/juggler-app/tests/qa_scenario_seed.rs` |
| Step timing | `ui/src/lib/walk-timing.ts` |
| Personas | `personas/` (AJ-E3307) |
