# Evidence manifest: AJ-E2316

**Task:** `docs/tasks/E23-session-runtime.md`, AJ-E2316: the rail's Pool lens capture waits for the rail to be still (a CI flake on bde5fa4d8, 91-pools). Commit 3b1e3caf0.

**Run:** 2026-11-23 (Opus), on the host, through `scripts/heavy.sh`: `cd ui && playwright test tests/visual/pools-gallery.spec.ts -g "Pool lens" --repeat-each=3 --workers=1` (default config, Chromium). The two gallery captures it rewrote were put back afterwards.

## By requirement

R1: PASS — `railStill(page)` in `ui/tests/visual/fixtures.ts` polls every `[data-line-key]` line's place until two reads agree (200–500 ms apart, up to 10 s), then needs no `rail-new-above` pill. Read in the code, and exercised: in all 6 runs (3 per theme, 1280x800), both waits returned and the capture was taken.
R2: PASS — `pools-gallery.spec.ts` "the rail's Pool lens" calls `railStill` before `hoverRailGroup` and again before `page.screenshot` (lines 104 and 112). In all 6 runs, the hover and the screenshot came after the rail settled, with no timeout and no lines still moving.

**Not this item's, found by the same run:** each of the 6 runs then failed its last step, `axe(page, "rail by pool …")`, on `label-content-name-mismatch: button[data-testid="feed-facet"]` (`ui/src/components/HomeFeedFilters.tsx`). That is a serious axe finding on Home feed's facet button, on today's tree. It is after both of this item's waits and outside the rail, so it belongs to the feed's owner, and CI's visual gate fails on it until it is fixed.

## Redaction checklist

- [x] No credentials, tokens, or keys here
- [x] No captures kept (the run's two were restored)
- [x] No real names or home paths
