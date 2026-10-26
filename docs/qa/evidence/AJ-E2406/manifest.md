# Evidence manifest — AJ-E2406

Task: `docs/tasks/E24-processes.md` AJ-E2406, Privacy lines, guide, and the processes lab. Branch `e24-processes`. Status: **IN-PROGRESS**.

## What exists now

Privacy lines `process-sightings` and `observed-stop`, and `this-mac-other-apps` extended to Processes' "Include other apps" (`ui/src/lib/trust-claims.json`, `docs/guide/trust.md`); the guide's `docs/guide/processes.md` (views, states, find, stop, another app's leftovers, automatic cleanup, the snapshot, what is kept) and `docs/guide/this-mac.md` pointing at it; the live checklist `docs/qa/processes-lab.md` (lab-started processes only).

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 every Privacy line names tests that exist and pass | `cargo test -p juggler-app --test trust` | each named test exists; pass | pass (14) | PASS | — | 2026-10-26 | agent |
| Req.1 the lines' own tests | `cargo test -p juggler-core --test sightings_storage`; `cargo test -p juggler-app --test observed_stop --test processes_graph`; `cargo test -p juggler-app --lib processes`; `pnpm exec vitest run src/screens/processes-page.test.tsx` | pass | pass | PASS | — | 2026-10-26 | agent |
| Req.2 the guide in plain words, its links whole | `just terminology` (terminology, plain-words, docs links) | OK | OK (26 files) | PASS | — | 2026-10-26 | agent |
| Req.2 the guide matches the page | read against the page and `_gallery/90-processes` | every named control exists with that word | checked by reading | PASS | — | 2026-10-26 | agent |
| Req.3 the lab checklist touches only what the lab starts | `docs/qa/processes-lab.md` (rules section; ports 8765–8767; pid-and-name listings only) | as written | written, words matched to the page | PASS | — | 2026-10-26 | agent |
| Req.3 the lab run on a real Mac | `docs/qa/processes-lab.md` steps 1–12, recorded in `docs/qa/evidence/AJ-E2406/lab-1-<date>.md` | every box ticked | not run yet | UNPROVEN | — | 2026-10-26 | agent |
| Req.4 manifests for AJ-E2401–E2405 and E2407 | `ls docs/qa/evidence/AJ-E240*/manifest.md`; `./scripts/evidence_check.sh` | all present; check OK | present | PASS | — | 2026-10-26 | agent |
| Visual: the Privacy page lines | the Privacy page reads `trust-claims.json` (the gallery's privacy captures, `84-privacy`) | the new text shows | not re-captured here | UNPROVEN | — | 2026-10-26 | agent |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (none: fixtures only)
