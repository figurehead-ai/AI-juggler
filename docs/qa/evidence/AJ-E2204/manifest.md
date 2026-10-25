# Evidence manifest — AJ-E2204

The visual usage pages (AJ-E2204 to AJ-E2206) on the usage world
(`ui/tests/visual/fixtures/usage-world.ts`: seven profiles on four
accounts, every runway state, 120 days of synthetic usage). Captures in
`docs/qa/evidence/_gallery/94-usage/`.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Gallery | `cd ui && npx playwright test tests/visual/usage-gallery.spec.ts` | every state renders; axe 0 critical/serious | 33 passed: Usage, a project's Overview and Usage, a profile's page, Accounts at 1280×800, 1680×1050, 2000×1100, dark and light | PASS | c2ec785c | 2026-10-25 | agent |
| 200 % zoom | same spec, `200 % zoom` | no page-level sideways scrolling | Usage and Accounts at 640 px, scale 2, both themes | PASS | c2ec785c | 2026-10-25 | agent |
| Instant | same spec, `the first paint draws the frame` | frame under 500 ms; reopen under 200 ms | first open: frame 317 ms (the screen's code loading), figures 439 ms; reopen: 23 ms (`time-to-first-content.json`) | PASS | c2ec785c | 2026-10-25 | agent |
| Unit | `cd ui && npx vitest run` | all pass | 126 files, 970 tests | PASS | c2ec785c | 2026-10-25 | agent |

## Binary artifacts

`docs/qa/evidence/_gallery/94-usage/*.png` (each window as shown; the
`-full` captures of the whole page are kept beside them locally).

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content
