# The Floor's budgets, 2026-11-04-first-run

Read by `scripts/floor_harness.py` on the release build with the harness
(AJ-E3513 req. 3), on the replica company: 7 Apps, 0 sessions. Budgets: SPEC §9.7.

Machine: Mac17,15, Apple M5 Ultra, 96 GB, macOS 27.0.1.
CPU is the whole app's, less the app alone; memory is its footprint (the GPU's
surfaces included) less before the Floor opened.

| Case | Budget | Measured | |
|---|---|---|---|
| Open, nothing moving | 0 frames after the first, but for data news | 12 frames in 60 s, 0 for data news | **over** |
| Open, nothing moving | < 0.1% of one core | -1.682% the Floor's own (the whole app -1.68% over a window alone) | within |
| Animating, Full | ≤ 2.0% of one core | 0.44% the Floor's own (the whole app +0.44% over) | within |
| Animating, Full | ≤ 1.5 ms of CPU a frame | 0.94 ms (2460 frames) | within |
| Animating, Calm | ≤ 2.0% of one core | 0.74% the Floor's own (252 frames) | within |
| Covered | 0 frames within 1 s | 239 after the first second (8 in it) | **over** |
| Minimised | 0 frames within 1 s | 0 after the first second (0 in it) | within |
| Closed | 0 frames | 0 | within |
| Opening | first frame ≤ 400 ms | 717 ms | **over** |
| A data change | on screen ≤ 100 ms | worst 30749.0 ms of 10 | **over** |
| Memory added, open and still | ≤ 150 MB | +148 MB | within |
| Memory added, animating (the Floor's own) | ≤ 250 MB | +675 MB at its highest; Metal's working memory on this Mac not recorded (the whole app +675 MB) | **over** |
| Memory once closed | no growth across repeated opens and closes | raw +93 MB once closed; no repeated cycles recorded | **over** |
| Building the scene | ≤ 50 ms | 5.41 ms on average | within |

## Every case

| Case | Seconds | CPU (% of one core) | Frames | In the first second | Drawing a frame | Footprint |
|---|---|---|---|---|---|---|
| Never opened (the app alone) | 60 | 3.480 | 0 | 0 | 0.00 ms | 53 MB |
| Open, at rest (Motion Off) | 60 | 1.798 | 12 | 1 | 1.50 ms | 201 MB |
| Animating, Calm | 60 | 4.221 | 252 | 4 | 1.37 ms | 707 MB |
| Animating, Full (5 minutes) | 300 | 3.916 | 2460 | 8 | 0.94 ms | 712 MB |
| Covered by another window | 30 | 4.055 | 247 | 8 | 0.72 ms | 713 MB |
| Minimised | 30 | 1.662 | 0 | 0 | 0.00 ms | 316 MB |
| Closed | 30 | 2.094 | 0 | 0 | 0.00 ms | 146 MB |

Memory while animating Full: 30 s: 716 MB, 150 s: 728 MB, 290 s: 712 MB (before opening: 53 MB).
A data change on screen, 10 tries: 30749.0, 30749.0, 30749.0, 30749.0, 30749.0, 30749.0, 30749.0, 30749.0, 30749.0, 30749.0 ms.
