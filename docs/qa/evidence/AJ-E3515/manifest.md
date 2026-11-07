# Evidence manifest — AJ-E3515 Labels by zoom

Built in d16811b95 (flags, bands, overlay). Requirement 1 amended there
(bands in points on a 640-point view; far below 14). Tests run on
`release/0.4.1` at the commit in each row; captures drawn offscreen on this
Mac's GPU (Metal) from the 0.4.1 company fixture (generic names only).

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 band from a desk's size, 15% hysteresis | `cargo test -p juggler-floor flags::tests::the_band_follows_a_desk_s_size_and_holds_near_its_edges`; `cargo test -p juggler-floor-render --test golden desk_sizes_by_zoom -- --ignored --nocapture` | close ≥ 28, middle 14–28, far < 14 pt on a 640-pt view; no flip inside ±15% | test passes; desk sizes by zoom: 0.3× home 63.0 pt and 0.6× 31.5 (close), home 18.9 and 1.3× 14.5 (middle), 1.6× 11.8 (far); identical at 1280×800, 2560×1600 and 1024×700 | PASS | d16811b95 | 2026-11-06 | Claude (self) |
| R2 close: no overlay labels but balloons and the card | `the_label_bands` (`AJ_FLOOR_EVIDENCE=… cargo test -p juggler-floor-render --test golden the_label_bands -- --ignored`) → `labels-close-pixel.png` | no flag or name flag drawn | only the state balloons over heads, the switcher, View, the lift and the hint line | PASS | d16811b95 | 2026-11-06 | Claude (self) |
| R3 middle: a flag over each desk that needs you, one name flag per area | `labels-middle-pixel.png`, `labels-middle-full.png`; `flags_never_overlap_never_lose_a_count_and_stand_over_their_place` | flags only over waiting desks, icon + word; one name per area; calm desks none | flags stand on short poles over the asking, stuck and crashed desks; one name flag per area; idle desks carry none | PASS | d16811b95 | 2026-11-06 | Claude (self) |
| R4 far: one flag per App that needs you, merged counts | `labels-far-pixel.png`, `labels-far-full.png`, `labels-building-pixel.png`; `flags_never_overlap_never_lose_a_count_and_stand_over_their_place` | one flag per waiting App, fixed text size, App colour on the pole; overlaps merge into a count | a flag per App that needs you ("2 WAITING WORKSHOP TOOL · WORK", "3 WAITING 2 PLACES" for a merge…), fixed size, App colour bar on each; the app's room marks in its words and tones: "OUT" in danger red with an empty gauge, "LOW · 14% LEFT" in amber with a quarter-full one (since 0.4.1's Out/Low match; `scene::tests::out_and_low_say_what_the_app_says`); calm Apps none | PASS | d16811b95 | 2026-11-06 | Claude (self) |
| R5 priority placement, never aside, joins a neighbour's count | `flags::tests::placement_holds_over_random_crowds`, `the_most_urgent_flag_keeps_its_place`, `flags_never_overlap_never_lose_a_count_and_stand_over_their_place`, `a_name_without_room_waits_for_a_closer_zoom`, `in_the_building_a_flag_hangs_below_rather_than_climb_a_floor` | Crashed > Asks > Stuck > Hot > Done > Out; no overlap; total count preserved | 400 seeded random crowds (1–24 flags, 20% names, sizes 40–140 px): no overlap, nothing under the taken band, every count shown, each flag straight over its place (or held 2 px inside the screen edge), a flag joins only one as urgent or more; on one spot the crash keeps its place and the others join it | PASS | d16811b95 | 2026-11-06 | Claude (self) |
| R6 never colour alone | captures above; `the_band_follows…` fixture flags | icon, word and colour on every flag | every flag has its icon (! ✕ ? z ✓), its word (ASKING, WAITING, OUT…) and its colour | PASS | d16811b95 | 2026-11-06 | Claude (self) |
| Gate: placement property tests | `cargo test -p juggler-floor flags` | all pass | 6 passed (the random-crowd property test added with this manifest) | PASS | this commit | 2026-11-06 | Claude (self) |
| Gate: golden frames per band | `the_gallery_of_golden_frames` (labels in every frame); `area_chips_never_overlap` | baselines from CI's lavapipe image | `area_chips_never_overlap` passes locally; the gallery's new baselines come from the CI floor-gallery artifact when the release pushes | UNPROVEN until CI | — | 2026-11-06 | Claude (self) |

## Binary artifacts

`labels-{close,middle,far,building}-{pixel,full}.png` (1280×800 at 2×,
offscreen, day light): the same company at each band in both looks.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (fixture only)
- [x] Paths hashed or generalised where they reveal private layout (none shown)
- [x] Transcript excerpts scrubbed of personal content (generic fixture names: Workshop tool, Studio app…)
