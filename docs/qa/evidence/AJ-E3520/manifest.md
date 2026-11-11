# Evidence manifest — AJ-E3520 Day and night from the Mac's clock

Built in d16811b95 (`juggler-floor::daylight`, platform `time_zone` and
`zone_table`, the app's `light.rs`). Golden hour retuned with this
manifest: at its low sun the floors read barely warmer than dusk; the fill
is now warm and the sun a little higher, so the floor reads beige and the
back wall orange with long wall shadows.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 sun from the zone's coordinates (zone.tab at run time) and the clock, NOAA | `cargo test -p juggler-floor daylight`; `cargo test -p juggler-platform a_zone_is_named_by_its_link` | known positions within 1° | Greenwich at midsummer noon 62° (within 1°, due south within 5°), at midwinter noon 15°, at midnight below the horizon; New York's coordinates read from a zone.tab; the zone named from /etc/localtime's link | PASS | d16811b95 | 2026-11-06 | Claude (self) |
| R2 steps every 10 minutes (one frame); 0 frames at rest | `cargo test -p juggler-app --lib features::floor::light`; the budget record `docs/qa/floor/budgets-2026-11-06-second.md` | a step only on a 10-minute boundary; 0 frames in 60 s at rest | tests pass; at rest 0 frames in 60 s on the release build | PASS | d16811b95 | 2026-11-06 | Claude (self) |
| R3 day, golden hour, dusk, night; signals keep contrast | `light-{day,golden,dusk,night}-pixel.png`, `light-golden-full.png`, `light-night-full.png`; `the_light_moves_from_day_through_dusk_to_night` | each time distinct; flags and balloons at full contrast | day bright and neutral; golden hour warm (beige floor, orange back wall, long shadows); dusk violet and dim; night dark with lamps and screens lit; every flag and balloon unchanged | PASS | this commit | 2026-11-06 | Claude (self) |
| R4 Follow my clock (default), Always day, Always night | `always_day_and_night_ignore_the_clock_and_no_zone_keeps_the_day`, `the_light_follows_the_clock_and_steps_every_ten_minutes`; View › Light in the walk | clock by default; the two fixed choices ignore it | tests pass; the menu items in `../AJ-E3519/manifest.md` | PASS | d16811b95 | 2026-11-06 | Claude (self) |
| Gate: sun position tests | `cargo test -p juggler-floor daylight` | all pass | 4 passed | PASS | this commit | 2026-11-06 | Claude (self) |
| Gate: budget run at rest | `scripts/floor_harness.py` (release build, AIJ_NO_KEYCHAIN=1) | 0 frames at rest | 0 frames in 60 s (both looks) | PASS | fd2637851 | 2026-11-06 | Claude (self) |

## Binary artifacts

`light-{day,golden,dusk,night}-pixel.png`, `light-{golden,night}-full.png`
(1280×800 at 2×).

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (fixture only)
- [x] Paths hashed or generalised where they reveal private layout (none shown)
- [x] Transcript excerpts scrubbed of personal content (generic fixture: Workshop tool, Studio app…)
