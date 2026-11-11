# The Floor's budgets in Full resolution (D-80), 2026-11-06

Read by `scripts/floor_harness.py` on the release build with the harness
(AJ-E3513 req. 3), on the replica company: 9 Apps, 340 sessions. Budgets: SPEC §9.7.

Machine: Mac17,15, Apple M5 Ultra, 96 GB, macOS 27.0.1.
Window: 1278 × 798 device pixels at scale 1; drawn at those pixels, edges smoothed by a post-pass (no MSAA).
CPU is the whole app's, less the app alone; memory is its footprint (the GPU's
surfaces included) less before the Floor opened.

| Case | Budget | Measured | |
|---|---|---|---|
| Open, nothing moving | 0 frames after the first, but for data news | 61 frames in 60 s, 0 for data news | **over** |
| Open, nothing moving | < 0.1% of one core | 0.313% the Floor's own (the whole app +1.30% over a window alone) | **over** |
| Animating, Motion Full | ≤ 2.0% of one core | 0.65% the Floor's own (the whole app +1.47% over) | within |
| Animating, Motion Full | ≤ 1.5 ms of CPU a frame | 0.71 ms (2588 frames) | within |
| Animating, Calm | ≤ 2.0% of one core | 0.27% the Floor's own (241 frames) | within |
| Covered | 0 frames within 1 s | 0 after the first second (0 in it) | within |
| Minimised | 0 frames within 1 s | 0 after the first second (0 in it) | within |
| Closed | 0 frames | 0 | within |
| Opening | first frame ≤ 400 ms | 308 ms | within |
| A data change | on screen ≤ 100 ms | worst 15.9 ms of 10 | within |
| Memory added, open and still | ≤ 150 MB | +113 MB | within |
| Memory added, animating (the Floor's own) | ≤ 250 MB | +103 MB at its highest; Metal's working memory on this Mac 630 MB beside it (the whole app +733 MB) | within |
| Memory once closed | no growth across repeated opens and closes | +13 MB over the last 3 of 6 more cycles; raw +79 MB once closed (the system's allocator keeps freed blocks for reuse) | within |
| Opened and closed 6 more times | the last 3 add ≤ 15 MB (no leak) | 146 → 168 → 174 → 175 → 184 → 187 MB (+13 over the last 3) | within |
| Building the scene | ≤ 50 ms | 6.58 ms on average | within |

## Every case

| Case | Seconds | CPU (% of one core) | Frames | In the first second | Drawing a frame | Footprint |
|---|---|---|---|---|---|---|
| Never opened (the app alone) | 60 | 5.578 | 0 | 0 | 0.00 ms | 60 MB |
| Open, at rest (Motion Off) | 60 | 6.876 | 61 | 0 | 2.82 ms | 173 MB |
| Animating, Calm | 60 | 6.910 | 241 | 4 | 0.57 ms | 752 MB |
| Animating, Motion Full (5 minutes) | 300 | 7.044 | 2588 | 8 | 0.71 ms | 785 MB |
| Covered by another window | 30 | 4.190 | 0 | 0 | 0.00 ms | 201 MB |
| Minimised | 30 | 1.897 | 0 | 0 | 0.00 ms | 202 MB |
| Closed | 30 | 2.030 | 0 | 0 | 0.00 ms | 139 MB |

Memory while animating (Motion Full): 30 s: 752 MB, 150 s: 785 MB, 290 s: 793 MB (before opening: 60 MB).
A data change on screen, 10 tries: 11.237, 15.914, 10.07, 15.804, 10.798, 9.374, 9.756, 10.003, 9.847, 15.754 ms.

Opening, step by step (ms after it was asked for): asked 0, window 12, surface 26, settings and office 27, snapshot 32, scene 44, renderer 74, first frame 308.

After three more opens and closes: 187 MB (after the first close: 139 MB).

Memory before opening, biggest parts: TOTAL 60 MB, Malloc Small 37 MB, Malloc Large 4 MB, Malloc Metadata 3 MB, WebKit Malloc 3 MB, __DATA_DIRTY 2 MB.
Memory open and still, biggest parts: TOTAL 173 MB, Malloc Small 56 MB, Owned physical footprint (unmapped) (graphics) 54 MB, Malloc Large 29 MB, IOSurface 12 MB, Malloc Metadata 3 MB.
Memory animating, biggest parts: TOTAL 785 MB, Owned physical footprint (unmapped) (graphics) 630 MB, Malloc Small 58 MB, Malloc Large 45 MB, IOSurface 12 MB, Owned physical footprint (unmapped) 12 MB.
Memory closed, biggest parts: TOTAL 139 MB, Malloc Small 59 MB, Malloc Large 45 MB, IOSurface 12 MB, Malloc Metadata 3 MB, WebKit Malloc 3 MB.
Memory closed after three more opens, biggest parts: TOTAL 187 MB, Malloc Small 87 MB, Malloc Large 45 MB, IOSurface 32 MB, Malloc Metadata 3 MB, WebKit Malloc 3 MB.

What the window said at the end of each case: Never opened (the app alone): no answer yet; Open, at rest (Motion Off): no answer yet; Animating, Calm: seen; Animating, Motion Full (5 minutes): seen; Covered by another window: not seen; Minimised: not seen, minimised; Closed: seen.
