# The building view, 0.4.1 (the maintainer's "too ugly … one line a floor")

Rendered headless from the golden test's fixture company (generic names), 1280 × 800, by `AJ_BUILDING_FRAMES=<dir> cargo test -p juggler-floor-render --test golden the_building_view_frames -- --ignored`; checked by eye.

- `before-after.png`: left before (7a3718ecb), right after (18606c40d); rows: day in Pixel art, dusk in Full resolution, night in Pixel art.
- `after-*.png`: the new view by day, dusk and night, in both looks.

What changed: one summary line a floor in a column left of the building ("FLOOR 1 · 2 WAITING · 1 CRASHED · 1 LOW · 1 HOT", most urgent first, the app's colours and chip words; a click enters the floor, a hover lists its Apps with reset times) instead of a flag an App; the building framed right of the middle for it; a graded sky (violet and amber at dusk, deep blue at night); striped lawns; a cornice on the roofline; the sign on a frame with bulbs lit at night.

Cost: about 70 more items, in the building view only (14 sky bands, 22 lawn stripes, 28 bulbs, the cornice and the sign's frame); a floor's view is unchanged.
