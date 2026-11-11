# Evidence manifest — AJ-E3521 The reworked look: light, shadow, occlusion and grade

Built behind `Look { rework }` in 77694c3a2, the second pass (materials,
people with faces) in f400ebefa, made the default in bea0ef391 after the
maintainer's approval (D-86, "Yes let's go with new graphics"); budgets in
fd2637851. Before/after drawn offscreen (Metal) from the same company.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 soft shadows (eight taps on a turned disk), both looks | shader `shadow()` in `juggler-floor-render`; `close-0.4.1-full.png`, `floor-0.4.1-pixel.png` beside `close-0.4.0-*` | soft edges in Pixel art and Full resolution | soft-edged shadows under desks, chairs and people in both looks; 0.4.0's are hard | PASS | 77694c3a2 | 2026-11-06 | Claude (self) |
| R2 ambient occlusion from depth in the post pass, radius in metres | post shader AO; same captures | contact darkening where things meet the floor | desks, plants and walls sit on the floor with a darker contact; AO is off in the side-on tower (flat elevation) by design | PASS | 77694c3a2 | 2026-11-06 | Claude (self) |
| R3 warm sun over cool shade, a rim light, a grade keeping signals at full contrast | same captures; AJ-E3520's four times | warm lit sides, cool shade; balloons and flags unchanged | warm sunlit tops and cool blue shade; a rim on people; balloons and flags keep their exact colours (the overlay is drawn after the grade) | PASS | 77694c3a2 | 2026-11-06 | Claude (self) |
| R4 default after approval; golden frames re-approved with notes | `Look::default()` = v2 (D-86), `Look::classic()` kept for tests and rollback; the golden baselines re-made on lavapipe in CI's image; `../AJ-E3513/critique-2026-11-06.md` | default changed; baselines re-approved | the default is the new look (bea0ef391); all 29 golden frames re-made in it on lavapipe in CI's own image (1b6ef4a46, b06c64f37), each critiqued first with notes (four fixes before any became a baseline), and the suite passes against them on aarch64 and x86-64 | PASS | b06c64f37 | 2026-11-06 | Claude (self) |
| R5 budgets on both looks; any line over says so | `docs/qa/floor/budgets-2026-11-06{,-second,-full,-full-second}.md` (+ `-first-try`) | every line within, or "over" with its number | Pixel art (first and second pass) and Full resolution (first pass): every line within. Full resolution, second pass: "Memory once closed" and "Opened and closed 6 more times" **over**: +22 MB over the last 3 of 6 cycles (budget 15; 94 → 106 → 127 → 141 → 144 → 149 MB); the offscreen test `opening_and_closing_does_not_grow` (30 renderers opened and closed, both looks) asserts the last 10 add < 15 MB, and passes | PASS (the over line is stated with its number) | fd2637851 | 2026-11-06 | Claude (self) |
| Gate: golden frames | `cargo test -p juggler-floor-render --test golden` in CI's image | the baselines in the new look | re-made and compared, as R4 | PASS | b06c64f37 | 2026-11-06 | Claude (self) |
| Gate: the harness's budgets | `scripts/floor_harness.py --look second` (release build, `AIJ_NO_KEYCHAIN=1`) | records written | four records written | PASS | fd2637851 | 2026-11-06 | Claude (self) |

## Binary artifacts

`close-0.4.0-{pixel,full}.png` (before), `close-0.4.1-full.png`,
`floor-0.4.1-pixel.png` (after).

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (fixture only)
- [x] Paths hashed or generalised where they reveal private layout (none shown)
- [x] Transcript excerpts scrubbed of personal content (generic fixture)

## 2026-11-06: the streaks on a quiet floor (maintainer finding)

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| No pale streaks from the shadow map: everything drawn is inside it | `cargo test -p juggler-floor camera::tests::the_shadow_box_holds_all_that_is_drawn`; `AJ_FLOOR_EVIDENCE=… cargo test -p juggler-floor-render --test golden the_quiet_floor_captures -- --ignored` | every corner of the floor in view (or the tower) projects inside the map, for the sun and moon from any side at 3–89.5°; nothing outside the map is shaded from its edge | test passes (3 boxes × 5 elevations × 5 azimuths); before (`streaks-before-dusk-full.png`, a quiet floor at dusk, light from the west): pale parallel bands on the ground beside the building and a pale stripe over the lift, the map's edge stretched across what fell outside it; after (`streaks-after-dusk-full.png`): none | PASS | this commit | 2026-11-06 | Claude (self) |
