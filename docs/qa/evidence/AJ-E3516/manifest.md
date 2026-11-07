# Evidence manifest — AJ-E3516 One floor at a time: lift, tab counts, the building view

Built in 4f548a4b4 (lift, Page Up/Down, tab counts, zoom never switches)
and 94a5df403 (the building view as a cross-section on its street, from
the maintainer's 2026-11-06 finding; R3, R4 and R6 amended then). No lift
on a one-floor office since 0f25e4cf7. Captures drawn offscreen (Metal)
from the company fixture (generic names); the timed walk is `walk.md`
here; the budgets in the building view are the harness records named in R6.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 lift buttons with counts, hidden at the ends; Page Up/Down; the ride (a cut with Reduce Motion) | `cargo test -p juggler-floor driver::tests::lift_buttons_page_keys_and_the_view_button driver::tests::a_camera_move_draws_until_it_lands scene::tests::the_overlay_has_a_chip_per_floor_and_area_chips_from_afar`; `lift-and-tabs-full.png` | the button goes to its floor; no floor past the top or bottom; a 0.45 s ride, 1 frame with Reduce Motion; no lift on one floor | tests pass (ride 25–35 frames; Reduce Motion: 1 frame; a one-floor office draws no lift); on floor 1 the lift shows "▲ FLOOR 2" over this floor's number "1", and no down button | PASS | 0f25e4cf7 | 2026-11-06 | Claude (self) |
| R2 tabs show each floor's waiting count | `labels-*` captures (AJ-E3515), `lift-and-tabs-full.png`, `building-*.png` | each tab with a count when anything waits | "FLOOR 1 !6" in amber; FLOOR 2, with nothing waiting, none | PASS | 4f548a4b4 | 2026-11-06 | Claude (self) |
| R3 the building view is a cross-section on its street | `AJ_FLOOR_EVIDENCE=… cargo test -p juggler-floor-render --test golden the_building_captures -- --ignored` → `building-{day,dusk,night}-{pixel,full}.png`; before: `building-before-{day,night}-pixel.png`; `cargo test -p juggler-floor scene::tests::the_building_is_a_cross_section_on_its_street` | facade cut away; slabs; side walls with windows lit at night; the shaft through every floor; roof with plant, antenna and sign; lobby with doors, reception, the lift; each App an office in its colour with its logo, desks and people, lit in use, dark and dusty when not; outside: ground, sidewalk, street, trees, bushes, lamps lit at night, cars, a bike rack; sun or moon, stars at night, clouds drifting only while animating | all of it in both looks: ten offices on floor 1 and one on floor 2, each in its colour with its logo and a person per session posed by state (the asker waving), the "Plain Tool · Old" office dark; the lobby's WELCOME wall, reception with someone behind it, the glass doors, bench and plants, the lift cab; the water tank, two units, the antenna's red light and the "AI JUGGLER" sign; three parked cars, the bike rack, four lamps beside the building (none in front of the cut face); by day blue glass windows, the sun behind clouds; at night warm lit windows, lamps pooling light on the sidewalk, the moon, stars and dark clouds. Test: four night-only lamp lights; a still building view isn't moving and draws its sky; clouds move it only with Motion on; no sky on a floor | PASS | 94a5df403 | 2026-11-06 | Claude (self) |
| R4 floor names and counts; a flag on each office that needs you; hover names an office; a click enters its floor | `scene::tests::the_building_is_a_cross_section_on_its_street` (each office's pick leads to its floor and is named for its App; rooms carry their count); `area_chips_never_overlap` (the usual and a crowded company, the building view among its views, at 1280×800, 1024×700 and 640×400); `building-*.png`; the walk (`walk.md`) | flags placed as the far band's: never over each other or a floor's name, no count lost | test passes; "FLOOR 1 !6" and "FLOOR 2" at the floors' left; a flag per office that needs you: "2 ASKING WORKSHOP TOOL · WORK", "1 ASKS STUDIO APP · WORK", "2 CRASHED INDUSTRIAL TOOL · HOME", "1 ASKS LAST TOOL · SPARE", and three that found no room above hang below their place ("1 STUCK GARAGE TOOL · LAB", "1 DONE CAMPUS TOOL · LAB", "1 OUT LAB TOOL · OUT"), over part of floor 1's offices; in AI Juggler Dev (walk.md): hovering an office outlines it with the hand and "CLAUDE CODE · CLAUDE-MAX-2 / CLOSED · CLICK TO GO IN"; a click on it enters Floor 2 (620 ms with the tool's pauses), the lobby Floor 1 | PASS | 94a5df403 | 2026-11-06 | Claude (self) |
| R5 zoom never switches floor/building | `cargo test -p juggler-floor camera::tests::zooming_never_switches_between_a_floor_and_the_building` | focus unchanged at every zoom | passes | PASS | 4f548a4b4 | 2026-11-06 | Claude (self) |
| R6 budgets in the building view, both looks; 0 frames at rest | `scripts/floor_harness.py run --home /tmp/aij-replica --view building --detail {pixel,full}` (release build, `AIJ_NO_KEYCHAIN=1`) | SPEC §9.7 and D-79's budgets | to be measured: the harness needs the one AI Juggler Dev slot, held by the 0.4.1 session's lab app (asked for a window) | UNPROVEN (pending the slot) | — | 2026-11-06 | Claude (self) |
| Gate: driver tests for buttons and keys | `cargo test -p juggler-floor driver` | all pass | 10 passed | PASS | 82f0bfb60 | 2026-11-06 | Claude (self) |
| Gate: scene tests for the building | `cargo test -p juggler-floor scene` | all pass | pass | PASS | 94a5df403 | 2026-11-06 | Claude (self) |
| Gate: golden frames of the building view | the gallery's building frame | CI lavapipe baseline | baselines come from the CI floor-gallery artifact when the release pushes | UNPROVEN until CI | — | 2026-11-06 | Claude (self) |

## Binary artifacts

`building-{day,dusk,night}-{pixel,full}.png` (the cross-section),
`building-before-{day,night}-pixel.png` (0.4.0's flat tower, for
comparison), `lift-and-tabs-full.png` (1280×800 at 2×).

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (fixture only)
- [x] Paths hashed or generalised where they reveal private layout (none shown)
- [x] Transcript excerpts scrubbed of personal content (generic fixture)
