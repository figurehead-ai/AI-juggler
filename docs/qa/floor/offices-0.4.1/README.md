# The offices, 0.4.1 (the maintainer: "the offices still read flat and empty")

Rendered headless from the golden test's fixture company (generic names), 1600 × 1000, by `AJ_OFFICE_FRAMES=<dir> cargo test -p juggler-floor-render --test golden the_office_frames -- --ignored`; checked by eye.

- `before-after.png`: left before (cce7d9b1e), right after (3dae58e68). Rows: the floor by day (Full resolution); the floor at night (Pixel art); a busy office (Workshop) close up; an unused office (Plain Tool, 12 days closed) close up; the back wall of Campus and Studio by day; the same at night.
- `after-*.png`: the new look, one frame each.

What changed:

- **Back walls in the App's colours.** The walls between the windows carry a poster (in the App's accent), a wall screen (showing code while the office is lit, off when it's unused) and a pin board, in turn. Before, posters were picked at random, a third of slots were empty, and boards sat over the windows. A low bookcase sits under each window, with books in the App's colour family and a box and a small plant on top. Front-row offices, which have no back wall, get a sideboard by their divider with binders and a plant.
- **A ceiling light strip in every office.** In a lit office it glows warm, with a soft pool of light on the floor below; in an unused one it hangs dark. It runs along the back wall, and in the front row behind the sign, so it never hides the App's name. Offices without pendants also get its light.
- **Unused offices are obvious.** A shade lies over the floor of an office closed over an hour (deeper after a week), and cobwebs hang in its corners after two days. The dust film was a pale beige that lit dimmed furniture back up to light grey; it is now as dim as what it lies on, so an unused office reads dim and dusty, not just grey. Its logo and sign stay readable.
- **More furniture:** the bookcases, sideboards and plants on them, and the wall screen as each office's second screen. Corner plants were already in every office.
- Fake soft dark only: true AO in the renderer is recorded as a follow-up in `docs/research/floor-04x.md` §12, not part of 0.4.1.

Costs, headless on this Mac, release build, from `the_second_pass_frames`' cost table (`AJ_SHOTS=costs`), before → after:

| | before | after |
|---|---|---|
| Still parts (classic / second pass) | 3786 / 3873 | 4229 / 4316 (+12%) |
| Moving parts | 1681 / 1942 | unchanged |
| Scene build, ms | 5.19 / 4.92 | 5.25 / 4.83 |
| CPU a frame, ms | 0.044 / 0.050 | 0.044 / 0.051 |
| GPU a frame, Retina, Pixel art (second pass), ms | 0.92 | 1.08 |
| GPU a frame, Retina, Full (second pass), ms | 1.59 | 1.68 |
| GPU a frame, wide 3440 × 1410, Full (second pass), ms | 1.76 | 1.84 |

CPU is unchanged and the GPU frame grows by under 0.2 ms. That is far inside SPEC §9.7's budgets: ≤ 1.5 ms CPU a frame, and the building view's harness run measured 15 of 15 within. The harness's own budget run on the release build needs the Dev slot; it runs with the RC.
