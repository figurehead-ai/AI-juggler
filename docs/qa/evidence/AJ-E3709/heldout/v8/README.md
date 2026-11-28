# Held-out sets, harness round v8 (AJ-E3830 / AJ-E3831)

- **Written by:** the blind writer (a Claude session that never saw the prompts, the harness code or any results).
- **Date:** 2026-11-26. Written blind.
- **Counts:** `watch-v8.json` 60 requests (45 English, 15 German); `turns-v8.json` 50 turns (36 English, 8 German, 3 French, 3 Spanish).
- **Watch mix:** 10 exclusions, 6 numeric thresholds, 8 other unsupported (all unsupported, with a nearest option); 6 two-name requests, 6 tricky names; 8 with quiet-hours words; the rest supported across ADR-0036 section 2.2 (hits_limit, idle_for, stuck, runway_below, finishes and fails, asks, starts, ends, runway_resets).
- **Lifetimes:** gold uses `once`, `today` and `always`. An unsaid lifetime is gold `today`. "today" resolves against 2026-11-18.
- **Turns:** mostly multi-clause (outcome or decision plus side details, file changes, step counts that are not test counts, reverted work, questions, tests passed, failed or not run).
- All names are fictional and generic; paths are project-relative.

**Gold lifetimes normalised (Opus, 2026-11-26, before any run):** `today` → `until:2026-11-18T23:59:00Z` and `always` → `until_removed`, the forms v5r3's scorer reads; `once` unchanged. Nothing else edited.
