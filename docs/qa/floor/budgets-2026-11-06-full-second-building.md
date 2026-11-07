# The Floor's budgets in Full resolution (D-80), 0.4.1's second pass (materials and faces), the building view (AJ-E3516's cross-section), 2026-11-06

Read by `scripts/floor_harness.py` on the release build with the harness
(AJ-E3513 req. 3), on the replica company: 9 Apps, 340 sessions. Budgets: SPEC §9.7.

Machine: Mac17,15, Apple M5 Ultra, 96 GB, macOS 27.0.1.
Window: 1278 × 798 device pixels at scale 1; drawn at those pixels, edges smoothed by a post-pass (no MSAA).
CPU is the whole app's, less the app alone; memory is its footprint (the GPU's
surfaces included) less before the Floor opened.

| Case | Budget | Measured | |
|---|---|---|---|
| Open, nothing moving | 0 frames after the first, but for data news | 2 frames in 60 s, 2 for data news | within |
| Open, nothing moving | < 0.1% of one core | 0.068% the Floor's own (the whole app -1.05% over a window alone) | within |
| Animating, Motion Full | ≤ 2.0% of one core | 0.53% the Floor's own (the whole app +0.62% over) | within |
| Animating, Motion Full | ≤ 1.5 ms of CPU a frame | 0.59 ms (2403 frames) | within |
| Animating, Calm | ≤ 2.0% of one core | 0.08% the Floor's own (0 frames) | within |
| Covered | 0 frames within 1 s | 0 after the first second (0 in it) | within |
| Minimised | 0 frames within 1 s | 0 after the first second (0 in it) | within |
| Closed | 0 frames | 0 | within |
| Opening | first frame ≤ 400 ms | 295 ms | within |
| A data change | on screen ≤ 100 ms | worst 29.3 ms of 10 | within |
| Memory added, open and still | ≤ 150 MB | +697 MB | **over** |
| Memory added, animating (the Floor's own) | ≤ 250 MB | +125 MB at its highest; Metal's working memory on this Mac 566 MB beside it (the whole app +691 MB) | within |
| Memory once closed | no growth across repeated opens and closes | +42 MB over the last 3 of 6 more cycles; raw +36 MB once closed (the system's allocator keeps freed blocks for reuse) | **over** |
| Opened and closed 6 more times | the last 3 add ≤ 15 MB (no leak) | 120 → 134 → 148 → 163 → 177 → 190 MB (+42 over the last 3) | **over** |
| Building the scene | ≤ 50 ms | 12.03 ms on average | within |

## Every case

| Case | Seconds | CPU (% of one core) | Frames | In the first second | Drawing a frame | Footprint |
|---|---|---|---|---|---|---|
| Never opened (the app alone) | 60 | 7.855 | 0 | 0 | 0.00 ms | 58 MB |
| Open, at rest (Motion Off) | 60 | 6.810 | 2 | 0 | 0.59 ms | 755 MB |
| Animating, Calm | 60 | 7.477 | 0 | 0 | 0.00 ms | 196 MB |
| Animating, Motion Full (5 minutes) | 300 | 8.471 | 2403 | 8 | 0.59 ms | 721 MB |
| Covered by another window | 30 | 8.379 | 0 | 0 | 0.00 ms | 193 MB |
| Minimised | 30 | 8.419 | 0 | 0 | 0.00 ms | 193 MB |
| Closed | 30 | 8.349 | 0 | 0 | 0.00 ms | 94 MB |

Memory while animating (Motion Full): 30 s: 749 MB, 150 s: 717 MB, 290 s: 721 MB (before opening: 58 MB).
A data change on screen, 10 tries: 12.977, 29.175, 20.091, 28.185, 28.293, 23.278, 29.31, 29.323, 12.264, 13.895 ms.

Opening, step by step (ms after it was asked for): asked 0, window 6, surface 19, settings and office 19, snapshot 22, scene 31, renderer 59, first frame 295.

After three more opens and closes: 190 MB (after the first close: 94 MB).

Memory before opening, biggest parts: TOTAL 58 MB, Malloc Small 35 MB, Malloc Large 4 MB, Malloc Metadata 3 MB, WebKit Malloc 3 MB, __DATA_DIRTY 2 MB.
Memory open and still, biggest parts: TOTAL 211 MB, Malloc Small 57 MB, Owned physical footprint (unmapped) (graphics) 54 MB, Malloc Large 45 MB, IOAccelerator (graphics) 17 MB, Owned physical footprint (unmapped) 9 MB.
Memory animating, biggest parts: TOTAL 721 MB, Owned physical footprint (unmapped) (graphics) 566 MB, Malloc Small 59 MB, Malloc Large 45 MB, IOSurface 12 MB, Owned physical footprint (unmapped) 10 MB.
Memory closed, biggest parts: TOTAL 94 MB, Malloc Small 40 MB, Malloc Large 20 MB, IOSurface 12 MB, Malloc Metadata 3 MB, WebKit Malloc 3 MB.
Memory closed after three more opens, biggest parts: TOTAL 190 MB, IOSurface 83 MB, Malloc Small 54 MB, Malloc Large 29 MB, Malloc Metadata 3 MB, WebKit Malloc 3 MB.

What the window said at the end of each case: Never opened (the app alone): no answer yet; Open, at rest (Motion Off): no answer yet; Animating, Calm: no answer yet; Animating, Motion Full (5 minutes): seen; Covered by another window: not seen; Minimised: not seen, minimised; Closed: seen.
