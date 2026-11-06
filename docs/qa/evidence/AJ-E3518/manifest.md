# Evidence manifest — AJ-E3518 Inactive Apps: lights off, dust and dust sheets

Built in d16811b95 (decay levels 1–5, covers follow the decay setting) on
the 0.4.2 look (D-86). Captures: one App ("cli:n") closed for 30 minutes,
6 hours, 3 days, 12 days and 40 days, by day and at night
(`AJ_FLOOR_EVIDENCE=… cargo test -p juggler-floor-render --test golden the_decay_and_daylight_captures -- --ignored`).

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 lamps and screens off; room light 80 / 55 / 40 / 25% by time; logo and sign readable | `cargo test -p juggler-floor scene::tests::an_unused_app_dims_by_how_long_and_decay_can_be_switched_off`; `decay-level{1..5}-day.png` | 0.8 at level 1 falling at each level to 0.25 at 5 | test passes (levels at 30 min, 12 h, 2, 10 and 40 days; light strictly falling, 0.8 and 0.25 at the ends); captures step down from level 1 to 5 with the screens dark from level 1, and the "PT" floor logo and its sign readable at level 5 | PASS | d16811b95 | 2026-11-06 | Claude (self) |
| R2 dust on upward surfaces from a day, growing | same test; `decay-level3-day.png` … `level5` | none under a day; more each level (dust_of: 0, 0, 0, 1, 2, 3 for levels 0–5) | test passes; the floor's checker fades under grey speckle from level 3, heaviest at 5 | PASS | d16811b95 | 2026-11-06 | Claude (self) |
| R3 dust sheets after a week; a box by the way in | `decay-level4-day.png`, `decay-level5-day.png`, `-night` | covers from level 4 (COVERED) | desks and screens under grey sheets at levels 4 and 5, two boxes by the way in | PASS | d16811b95 | 2026-11-06 | Claude (self) |
| R4 running again: lights first; covers go with the next snapshot | `scene::tests::an_unused_app_dims_by_how_long_and_decay_can_be_switched_off` (an open App: level 0) | level 0 once open; the scene is rebuilt from each snapshot | passes (assertion for the open App added with this manifest) | PASS | d16811b95 | 2026-11-06 | Claude (self) |
| R5 "Show unused Apps' decay" is the off switch | same scene test with decay off | only "closed" (level 1: lamps and screens off), no deeper dimming, dust or covers | passes (level 1 at every age with the switch off) | PASS | d16811b95 | 2026-11-06 | Claude (self) |
| Gate: scene tests per level | `cargo test -p juggler-floor scene` | all pass | pass | PASS | d16811b95 | 2026-11-06 | Claude (self) |
| Gate: golden frames | the gallery's frames | CI lavapipe baselines | from the CI floor-gallery artifact when the release pushes | UNPROVEN until CI | — | 2026-11-06 | Claude (self) |

## Binary artifacts

`decay-level{1..5}-day.png`, `decay-level5-night.png` (1280×800 at 2×,
Pixel art).

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (fixture only)
- [x] Paths hashed or generalised where they reveal private layout (none shown)
- [x] Transcript excerpts scrubbed of personal content (generic fixture: Workshop tool, Studio app…)
