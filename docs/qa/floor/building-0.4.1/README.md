# The building view, 0.4.1 (the maintainer's "too ugly … one line a floor")

Rendered headless from the golden test's fixture company (generic names), 1280 × 800, by `AJ_BUILDING_FRAMES=<dir> cargo test -p juggler-floor-render --test golden the_building_view_frames -- --ignored`; checked by eye.

- `before-after.png`: left before (7a3718ecb), right after (cf325be5a); rows: day in Pixel art, dusk in Full resolution, night in Pixel art.
- `after-*.png`: the new view by day, dusk and night, in both looks.

What changed (two rounds): one summary line a floor in a column left of the building ("FLOOR 1 · 2 WAITING · 1 CRASHED · 1 LOW · 1 HOT", most urgent first, the app's colours and chip words; a click enters the floor, a hover lists its Apps with reset times) instead of a flag an App; the building framed right of the middle for it; a graded sky (violet and amber at dusk, deep blue at night); striped lawns; a cornice on the roofline; the sign on a frame with bulbs lit at night; three windows a storey with mullions on the side walls; a real entrance (glass doors, a mullion, handles, a canopy, light behind at night); contact shadows under the cars, the trees and along the building's foot; the people a little larger; a tick from each floor line to the building; the building a little high in the frame, the lawn to the view's bottom edge.

Budgets (SPEC §9.7), measured on the release build with the harness on the replica, building view, quick: `budgets-pixel-quick.md` and `budgets-full-quick.md`: 15 within, 0 over in each (Pixel art: 0.51 ms of CPU a frame, first frame 281 ms, +107 MB open). About 120 more items, in the building view only; a floor's view is unchanged.
