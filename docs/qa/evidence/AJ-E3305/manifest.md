# Evidence manifest — AJ-E3305 Every 0.4 screen scored against the 14 principles

Task: `docs/tasks/E33-validated-by-use.md` AJ-E3305. The record is
`docs/qa/0.4-walks/design.md`: the first scoring (2026-11-05), what was done
with each no, 0.4.1's additions, and "The re-score on 0.4.1 (2026-11-07)".

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 a checklist per screen of the 14 principles, with captures at 1024 and 1280 in both themes | the gallery retaken on `release/0.4.1` (`pnpm exec playwright test` over the 15 gallery specs of the 24 families; `pools-gallery.spec.ts` now captures Agent access at 1024 too), opened on contact sheets; `design.md` "The re-score on 0.4.1": a row of 14 scores per family, 24 families and 4 new surfaces (the pool wizard, Accounts › Apps, the header, the Floor's looks), a Fit line per family | every family scored on every line, both sizes and themes | 28 rows; every capture family at 1280 and 1024 in both themes (91-pools' rail captures stay at 1280: the rail folds to icons below 1100 px) | PASS | b4bbd9eb9 | 2026-11-07 | Claude (self) |
| R1 line 14, walked by hand | the design preview in the browser, at 1280 dark and 1024 light, each family's screen opened and used as a user would (design.md, "Line 14, walked by hand") | each family walked | all 28 but the Floor's native window (the Floor session's walks) | PASS | b4bbd9eb9 | 2026-11-07 | Claude (self) |
| R2 a critique beside each gallery family; every no fixed or explained | each family's `critique.md` gains "0.4.1 re-score (2026-11-07)" saying what its retake shows and what changed, stamped (`scripts/critique_check.py --stamp`, check OK); fixes 9d96ccd06 (App room said once, empty-state step secondary, Usage's words, the preview's desktop order, the wizard's "(0.5)") | every line under 4 fixed or explained with its reason | six fixed this round; five under 4 explained, each with why and whose call (design.md, "Under 4, not fixed here") | PASS | 9d96ccd06 | 2026-11-07 | Claude (self) |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Captures use the preview's generic fixtures (`…@example.com`)
- [x] No conversation content
