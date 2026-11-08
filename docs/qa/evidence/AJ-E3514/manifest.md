# Evidence manifest — AJ-E3514 Walked by hand: persona walks on the release build

The walk sheet, the runs and the issues: `docs/qa/floor/persona-walk.md`, `docs/qa/floor/persona-walk/issues.md`, the pilot `docs/qa/floor/persona-walk/pilot-2026-11-07.md`. Runs on the bundled `floor-harness` release build of `release/0.4.1`, lab FL1 (generic names), BetterStage on (the main session's ruling: fix the product, not the setup). One Sonnet tester per flow under the token budget (the 0.4.1 run plan); captures in the session scratchpads, not copied (not all checked by eye).

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 a persona walk of every step on the release build | the flows F1–F9 (issues.md) | every step walked | F6, F7 pass; F1, F2, F4, F9 partly; F8 not reached (window manager); F3, F5 not walked (lab) | OPEN — needs a decision to defer F3, F5, F8 | 4fd141235 | 2026-11-08 | Claude (self) |
| R2 each step scored; under 4 to the maintainer; bugs fixed | the testers' records; the critic on low runs (2026-11-08 run 2 of F1) | nothing under 4 | lows recorded and on the list (`docs/qa/0.4-completion.md`); bugs found fixed (the door order 859150589, the rack and reset words 32083e36d, the title by kind 3fa4ff799, the count ace8790b0) | OPEN — the lows are design calls for the maintainer | 32083e36d | 2026-11-08 | Claude (self) |
| R3 the Builder's hand walk of every element | — | every element | not done in this round (the token budget) | OPEN | — | 2026-11-08 | Claude (self) |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Generic lab names only
- [x] No conversation content
