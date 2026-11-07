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
| Open, nothing moving | < 0.1% of one core | 0.114% the Floor's own (the whole app -0.28% over a window alone) | **over** |
| Animating, Motion Full | ≤ 2.0% of one core | 0.65% the Floor's own (the whole app +1.97% over) | within |
| Animating, Motion Full | ≤ 1.5 ms of CPU a frame | 0.73 ms (2404 frames) | within |
| Animating, Calm | ≤ 2.0% of one core | 0.02% the Floor's own (0 frames) | within |
| Covered | 0 frames within 1 s | 0 after the first second (0 in it) | within |
| Minimised | 0 frames within 1 s | 0 after the first second (0 in it) | within |
| Closed | 0 frames | 0 | within |
| Opening | first frame ≤ 400 ms | 294 ms | within |
| A data change | on screen ≤ 100 ms | worst 30.7 ms of 10 | within |
| Memory added, open and still | ≤ 150 MB | +137 MB | within |
| Memory added, animating (the Floor's own) | ≤ 250 MB | +107 MB at its highest; Metal's working memory on this Mac 582 MB beside it (the whole app +689 MB) | within |
| Memory once closed | no growth across repeated opens and closes | +6 MB over the last 3 of 6 more cycles; raw +71 MB once closed (the system's allocator keeps freed blocks for reuse) | within |
| Opened and closed 6 more times | the last 3 add ≤ 15 MB (no leak) | 133 → 135 → 144 → 145 → 148 → 150 MB (+6 over the last 3) | within |
| Building the scene | ≤ 50 ms | 10.79 ms on average | within |

## Every case

| Case | Seconds | CPU (% of one core) | Frames | In the first second | Drawing a frame | Footprint |
|---|---|---|---|---|---|---|
| Never opened (the app alone) | 60 | 8.007 | 0 | 0 | 0.00 ms | 58 MB |
| Open, at rest (Motion Off) | 60 | 7.725 | 2 | 0 | 0.75 ms | 195 MB |
| Animating, Calm | 60 | 8.755 | 0 | 0 | 0.00 ms | 195 MB |
| Animating, Motion Full (5 minutes) | 300 | 9.975 | 2404 | 8 | 0.73 ms | 736 MB |
| Covered by another window | 30 | 8.343 | 0 | 0 | 0.00 ms | 192 MB |
| Minimised | 30 | 6.385 | 0 | 0 | 0.00 ms | 201 MB |
| Closed | 30 | 8.446 | 0 | 0 | 0.00 ms | 129 MB |

Memory while animating (Motion Full): 30 s: 747 MB, 150 s: 715 MB, 290 s: 736 MB (before opening: 58 MB).
A data change on screen, 10 tries: 30.677, 28.009, 11.123, 14.037, 22.786, 26.689, 27.585, 26.169, 18.019, 27.285 ms.

Opening, step by step (ms after it was asked for): asked 0, window 6, surface 18, settings and office 18, snapshot 23, scene 32, renderer 58, first frame 294.

After three more opens and closes: 88 MB (after the first close: 129 MB).

Memory before opening, biggest parts: TOTAL 58 MB, Malloc Small 35 MB, Malloc Large 4 MB, Malloc Metadata 3 MB, WebKit Malloc 3 MB, __DATA_DIRTY 2 MB.
Memory open and still, biggest parts: TOTAL 195 MB, Malloc Small 56 MB, Owned physical footprint (unmapped) (graphics) 54 MB, Malloc Large 45 MB, IOAccelerator (graphics) 10 MB, IOSurface 8 MB.
Memory animating, biggest parts: TOTAL 736 MB, Owned physical footprint (unmapped) (graphics) 582 MB, Malloc Small 57 MB, Malloc Large 45 MB, IOSurface 12 MB, Owned physical footprint (unmapped) 11 MB.
Memory closed, biggest parts: TOTAL 129 MB, Malloc Small 61 MB, Malloc Large 45 MB, Malloc Metadata 3 MB, WebKit Malloc 3 MB, Stack 2 MB.
Memory closed after three more opens, biggest parts: TOTAL 88 MB, Malloc Small 60 MB, Malloc Large 4 MB, Malloc Metadata 3 MB, WebKit Malloc 3 MB, Stack 2 MB.

What the window said at the end of each case: Never opened (the app alone): no answer yet; Open, at rest (Motion Off): no answer yet; Animating, Calm: no answer yet; Animating, Motion Full (5 minutes): seen; Covered by another window: not seen; Minimised: not seen, minimised; Closed: seen.
