# Evidence manifest — AJ-E806 (gates + CI enforcement)

| Claim / Req ID | Command / fixture | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 gate vocabulary | justfile + QUALITY.md + CI | names identical in all three | verify/ui/evidence wired; visual/a11y/perf named with owners | PASS | (this PR) | 2026-10-13 | zcode-primary |
| Seeded unit failure | seeded-unit-red.log (+ removal green) | gate red, then green after removal | FAILED captured; green restored | PASS | (this PR) | 2026-10-13 | zcode-primary |
| Seeded visual failure | seeded-visual-red.log (tampered capture, byte compare) | compare red | non-zero detected | PASS | (this PR) | 2026-10-13 | zcode-primary |
| Seeded a11y failure | axe harness | gate red on seeded violation | DEFERRED → AJ-E804 (harness not yet built; gate named, not silently passed) | DEFERRED | — | 2026-10-13 | zcode-primary |
| Evidence in CI | `.github/workflows/ci.yml` verify job | evidence step present, no new job | step added inside verify | PASS | (this PR) | 2026-10-13 | zcode-primary |

## Notes
- a11y seeded failure is recorded DEFERRED with owning task AJ-E804 —
  the gate is named in QUALITY.md vocabulary now; its absence is
  explicit, never a silent pass (per the quarantine/deferral rule).
- Flake root cause FIXED (2026-10-18, AJ-E309): the recorded
  "light-theme first-scan contrast flash" (axe color-contrast on
  freshly booted light-theme dialogs, handled so far by the one
  confirmatory re-scan) came from `styles.css`'s reduced-motion rule
  setting `transition-duration: 0.01ms` on every element — with the
  default `transition-property: all`, every element transitioned its
  inherited text colour after boot (CDP: dialog `color` = dark ink
  while `--color-ink` = light ink). Now `transition: none !important`
  under reduced motion; the wizard dialog has the correct ink on the
  first frame (evidence: `docs/qa/evidence/AJ-E309/manifest.md`). The
  re-scan guard stays; no gate threshold changed.

