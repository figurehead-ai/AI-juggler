# The Floor's budgets, 0.4.1's second pass (materials and faces), the building view (AJ-E3516's cross-section), 2026-11-06

Read by `scripts/floor_harness.py` on the release build with the harness
(AJ-E3513 req. 3), on the replica company: 9 Apps, 340 sessions. Budgets: SPEC §9.7.

Machine: Mac17,15, Apple M5 Ultra, 96 GB, macOS 27.0.1.
Window: 1278 × 798 device pixels at scale 1; the scene drawn at its pixel size and scaled up (no MSAA).
CPU is the whole app's, less the app alone; memory is its footprint (the GPU's
surfaces included) less before the Floor opened.

| Case | Budget | Measured | |
|---|---|---|---|
| Open, nothing moving | 0 frames after the first, but for data news | 0 frames in 60 s, 0 for data news | within |
| Open, nothing moving | < 0.1% of one core | 0.058% the Floor's own (the whole app +1.65% over a window alone) | within |
| Animating, Motion Full | ≤ 2.0% of one core | 0.62% the Floor's own (the whole app +2.54% over) | within |
| Animating, Motion Full | ≤ 1.5 ms of CPU a frame | 0.70 ms (2403 frames) | within |
| Animating, Calm | ≤ 2.0% of one core | 0.04% the Floor's own (2 frames) | within |
| Covered | 0 frames within 1 s | 0 after the first second (0 in it) | within |
| Minimised | 0 frames within 1 s | 0 after the first second (0 in it) | within |
| Closed | 0 frames | 0 | within |
| Opening | first frame ≤ 400 ms | 298 ms | within |
| A data change | on screen ≤ 100 ms | worst 30.4 ms of 10 | within |
| Memory added, open and still | ≤ 150 MB | +101 MB | within |
| Memory added, animating (the Floor's own) | ≤ 250 MB | +122 MB at its highest; Metal's working memory on this Mac 557 MB beside it (the whole app +679 MB) | within |
| Memory once closed | no growth across repeated opens and closes | +10 MB over the last 3 of 6 more cycles; raw +71 MB once closed (the system's allocator keeps freed blocks for reuse) | within |
| Opened and closed 6 more times | the last 3 add ≤ 15 MB (no leak) | 135 → 136 → 137 → 139 → 143 → 147 MB (+10 over the last 3) | within |
| Building the scene | ≤ 50 ms | 8.98 ms on average | within |

## Every case

| Case | Seconds | CPU (% of one core) | Frames | In the first second | Drawing a frame | Footprint |
|---|---|---|---|---|---|---|
| Never opened (the app alone) | 60 | 7.176 | 0 | 0 | 0.00 ms | 59 MB |
| Open, at rest (Motion Off) | 60 | 8.823 | 0 | 0 | 0.00 ms | 160 MB |
| Animating, Calm | 60 | 6.475 | 2 | 0 | 0.82 ms | 194 MB |
| Animating, Motion Full (5 minutes) | 300 | 9.713 | 2403 | 8 | 0.70 ms | 706 MB |
| Covered by another window | 30 | 8.773 | 0 | 0 | 0.00 ms | 179 MB |
| Minimised | 30 | 5.123 | 0 | 0 | 0.00 ms | 179 MB |
| Closed | 30 | 7.416 | 0 | 0 | 0.00 ms | 130 MB |

Memory while animating (Motion Full): 30 s: 738 MB, 150 s: 705 MB, 290 s: 706 MB (before opening: 59 MB).
A data change on screen, 10 tries: 20.203, 29.976, 30.435, 15.225, 9.342, 10.522, 9.632, 11.772, 11.057, 12.099 ms.

Opening, step by step (ms after it was asked for): asked 0, window 16, surface 20, settings and office 21, snapshot 25, scene 36, renderer 68, first frame 298.

After three more opens and closes: 88 MB (after the first close: 130 MB).

Memory before opening, biggest parts: TOTAL 59 MB, Malloc Small 36 MB, Malloc Large 4 MB, Malloc Metadata 3 MB, WebKit Malloc 3 MB, Stack 2 MB.
Memory open and still, biggest parts: TOTAL 160 MB, Malloc Small 55 MB, Owned physical footprint (unmapped) (graphics) 45 MB, Malloc Large 29 MB, IOSurface 8 MB, Malloc Metadata 3 MB.
Memory animating, biggest parts: TOTAL 706 MB, Owned physical footprint (unmapped) (graphics) 557 MB, Malloc Small 58 MB, Malloc Large 45 MB, Owned physical footprint (unmapped) 9 MB, IOAccelerator (graphics) 9 MB.
Memory closed, biggest parts: TOTAL 130 MB, Malloc Small 63 MB, Malloc Large 45 MB, Malloc Metadata 3 MB, WebKit Malloc 3 MB, Stack 2 MB.
Memory closed after three more opens, biggest parts: TOTAL 88 MB, Malloc Small 56 MB, Malloc Large 9 MB, Malloc Metadata 3 MB, WebKit Malloc 3 MB, Stack 2 MB.

What the window said at the end of each case: Never opened (the app alone): no answer yet; Open, at rest (Motion Off): no answer yet; Animating, Calm: no answer yet; Animating, Motion Full (5 minutes): seen; Covered by another window: not seen; Minimised: not seen, minimised; Closed: seen.
