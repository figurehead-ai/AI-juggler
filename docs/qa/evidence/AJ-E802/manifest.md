# Evidence manifest — AJ-E802 (screenshot harness + states gallery)

| Claim / Req ID | Command / fixture | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 fixtures: 9 statuses | `ui/tests/visual/fixtures.ts` (allStatuses) | all 9 SessionStatus present | starting/running/waiting_user/stuck/quota_blocked/finished/stopped/failed/handed_over | PASS | (this PR) | 2026-10-13 | zcode-primary |
| Req.2 matrix | `pnpm --dir ui visual:capture` | 4 viewports × 2 themes × seed states | 41 captures (all cells present) | PASS | (this PR) | 2026-10-13 | zcode-primary |
| Req.3 gallery page | `scripts/gallery_index.sh` → `_gallery/index.html` | browsable, grouped, labelled | generated; NOT CAPTURED table included | PASS | (this PR) | 2026-10-13 | zcode-primary |
| Req.4 determinism | harness self-test (byte compare) | 0 bytes diff | byte-identical; PASS | PASS | (this PR) | 2026-10-13 | zcode-primary |
| Req.5 not-captured loud | `_gallery/not-captured.md` | every missing state listed with reason | density/loading/error/stale recorded → pending AJ-E401/E403 | PASS | (this PR) | 2026-10-13 | zcode-primary |
| Perf budget | full run | < 120 s | 13.9 s (41 tests); PNGs all < 500 KB; total 4.0 MB | PASS | (this PR) | 2026-10-13 | zcode-primary |
| D-16 themes | light captures render token layer | both themes captured | light verified visually (rail + panels) | PASS | (this PR) | 2026-10-13 | zcode-primary |

## Notes
- Runs against the production build (vite preview), never a live
  backend — mocked-bridge determinism per the task's handover note.
- Loading/error/stale/density states are NOT CAPTURED with owning task
  IDs (AJ-E401/E403); they will flip to captured as those land.
