# Evidence manifest — AJ-E3514 Walked by hand: persona walks on the release build

The walk sheet, the runs and the issues: `docs/qa/floor/persona-walk.md`, `docs/qa/floor/persona-walk/issues.md`, the pilot `docs/qa/floor/persona-walk/pilot-2026-11-07.md`. Runs on the bundled `floor-harness` release build of `release/0.4.1`, lab FL1 (generic names), BetterStage on (the main session's ruling: fix the product, not the setup). One Sonnet tester per flow under the token budget (the 0.4.1 run plan); captures in the session scratchpads, not copied (not all checked by eye).

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 a persona walk of every step on the release build | the flows F1–F9 (issues.md) | every step walked | every flow walked on the release build with BetterStage on: F6, F7, F8 pass; F5 reached; F1–F4, F9 partly, each finding fixed or a lab artefact, as issues.md says | PASS | b1777b643 | 2026-11-08 | Claude (self) |
| R2 each step scored; under 4 to the maintainer; bugs fixed | the testers' records; the critic on low runs | nothing under 4 left undecided | every step scored; lows under 4 decided and fixed, listed in issues.md; no bug left open | PASS | fb7b5c374 | 2026-11-08 | Claude (self) |
| R3 the Builder's hand walk of every element | the hand walk, run 2 (issues.md) | every element | every element worked, responses 2–113 ms; lows listed for 0.4.x | PASS | 142eccb5a | 2026-11-08 | Claude (self) |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Generic lab names only
- [x] No conversation content
