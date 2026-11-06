# Evidence manifest — AJ-E3516 One floor at a time: lift, tab counts, the tower

Built in 4f548a4b4 (tower, lift, Page Up/Down, tab counts, zoom never
switches); the driver test for the lift buttons and the View button added
with this manifest. Captures drawn offscreen (Metal) from the 0.4.2 company
fixture; the timed walk is in `walk.md` here.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 lift buttons with counts, hidden at the ends; Page Up/Down; the ride (a cut with Reduce Motion) | `cargo test -p juggler-floor driver::tests::lift_buttons_page_keys_and_the_view_button driver::tests::a_camera_move_draws_until_it_lands`; `lift-and-tabs-full.png` | the button goes to its floor; no floor past the top or bottom; a 0.45 s ride, 1 frame with Reduce Motion | tests pass (ride 25–35 frames; Reduce Motion: 1 frame); on floor 1 the lift shows "▲ FLOOR 2" (no count: nothing waits there) over this floor's number "1", and no down button | PASS | 4f548a4b4 | 2026-11-06 | Claude (self) |
| R2 tabs show each floor's waiting count | `labels-*` captures (AJ-E3515), `lift-and-tabs-full.png` | each tab with a count when anything waits | "FLOOR 1 !6" in amber; FLOOR 2 with nothing waiting shows none | PASS | 4f548a4b4 | 2026-11-06 | Claude (self) |
| R3 the building is a side-on tower | `tower-pixel.png`, `tower-full.png` (`the_label_bands`, building view) | a strip per floor, a room per App in its colour with its mark, dark when unused, a count per floor, the lobby | two storeys over the LOBBY; each App a room in its colour with its mark; the unused one dark; badges (! ✕ ? ✓ z) on rooms that need you; "!6" beside floor 1 | PASS | 4f548a4b4 | 2026-11-06 | Claude (self) |
| R4 hover lights a floor with its summary; a click enters | `driver::tests::clicks_open_doors_and_play_with_toys` (floor picks), the hand walk (`walk.md`) | lit strip + summary on hover; click enters | see walk.md | see walk.md | 4f548a4b4 | 2026-11-06 | Claude (self) |
| R5 zoom never switches floor/building | `cargo test -p juggler-floor camera::tests::zooming_never_switches_between_a_floor_and_the_building` | focus unchanged at every zoom | passes | PASS | 4f548a4b4 | 2026-11-06 | Claude (self) |
| Gate: driver tests for buttons and keys | `cargo test -p juggler-floor driver` | all pass | 10 passed | PASS | this commit | 2026-11-06 | Claude (self) |
| Gate: golden frames of the tower | the gallery's building frame | CI lavapipe baseline | baselines come from the CI floor-gallery artifact when the release pushes | UNPROVEN until CI | — | 2026-11-06 | Claude (self) |

## Binary artifacts

`tower-{pixel,full}.png`, `lift-and-tabs-full.png` (1280×800 at 2×).

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (fixture only)
- [x] Paths hashed or generalised where they reveal private layout (none shown)
- [x] Transcript excerpts scrubbed of personal content (generic fixture)
