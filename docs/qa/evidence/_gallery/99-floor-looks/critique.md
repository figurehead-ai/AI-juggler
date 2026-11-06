<!-- captures: 2057625cc085 -->
<!-- captures: pending -->
# 99-floor-looks: the Floor in Pixel art and in Full resolution (D-80)

Design critique (D-57) of the Floor's two looks, rendered by the golden-frame test (`AJ_D80_CAPTURE`) at 1280 × 800 on a Retina-scaled frame, the building view and floor 1 in each. The 3D Floor has one look in light and dark alike (its sky and light don't follow the app's theme); the setting that picks the look is in 99-floor-settings, in both themes.

## Screenshot notes

- `pixel-art-building-1280x800`: the building in Pixel art, both floors; floor 1's App labels stand below floor 2's slab, none over another, each over its own floor; long names whole ("INDUSTRIAL TOOL · HOME X1 14% HOT").
- `full-resolution-building-1280x800`: the same view at the frame's own pixels: smooth edges, sharp textures, the same labels in the same places.
- `pixel-art-floor-1-1280x800`: floor 1 in Pixel art; every label whole and inside the window ("ANOTHER TOOL · SPARE", "LAST TOOL · SPARE !1 14%" at the right edge), none under a ! or ✓ balloon.
- `full-resolution-floor-1-1280x800`: floor 1 at full resolution: the same scene, the same labels, outlines and the hover ring at Pixel art's weight.

## Checklist

| Check | Finding |
|---|---|
| Labels | No two overlap, none under a balloon, the switcher or the hint, all inside the window; a moved one keeps a 1 px stick in its area's colour (the golden test `area_chips_never_overlap` checks every view at three sizes, a crowded office included). |
| Floors | In the building view a label stays over its own floor: it moves down or aside before up and never onto another floor's slab. |
| Both looks | The same scene, labels, outlines and hover; only the pixels differ. |
| Plain words | Labels in the Floor's own capitals; long names end in ".." at 24 characters, never on the "·". |
| Cost | Measured beside each other (docs/qa/floor/budgets-2026-11-05.md and -full.md): within D-79 at the same window; on a large window Full resolution goes past the CPU budget, which the setting says. |

## Good

- Full resolution reads as the same office, just finer: nothing moves between looks.
- Labels now read as belonging to their floor in the building view.

## Weak

- Full resolution's cost grows with the window: on a large or Retina window it's over the Floor's 2% CPU budget while animating.

## Fixed

- Labels printed over each other in the building view and under balloons on floor 1, and two were cut at the right edge; fixed before these captures (the placement, the floor rule, the sticks).
