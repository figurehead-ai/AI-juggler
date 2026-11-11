# The Floor's budgets, 0.4.1's second pass (materials and faces), the building view (AJ-E3516's cross-section), 2026-11-08

Read by `scripts/floor_harness.py` on the release build with the harness
(AJ-E3513 req. 3), on the replica company: 0 Apps, 0 sessions. Budgets: SPEC §9.7.

Machine: Mac17,15, Apple M5 Ultra, 96 GB, macOS 27.0.1.
Window: 1278 × 798 device pixels at scale 1; the scene drawn at its pixel size and scaled up (no MSAA).
CPU is the whole app's, less the app alone; memory is its footprint (the GPU's
surfaces included) less before the Floor opened.

| Case | Budget | Measured | |
|---|---|---|---|
| Open, nothing moving | 0 frames after the first, but for data news | 1 frames in 12 s, 1 for data news | within |
| Open, nothing moving | < 0.1% of one core | 0.093% the Floor's own (the whole app +1.50% over a window alone) | within |
| Animating, Motion Full | ≤ 2.0% of one core | 0.43% the Floor's own (the whole app -0.48% over) | within |
| Animating, Motion Full | ≤ 1.5 ms of CPU a frame | 0.51 ms (486 frames) | within |
| Animating, Calm | ≤ 2.0% of one core | 0.31% the Floor's own (50 frames) | within |
| Covered | 0 frames within 1 s | 0 after the first second (0 in it) | within |
| Minimised | 0 frames within 1 s | 0 after the first second (0 in it) | within |
| Closed | 0 frames | 0 | within |
| Opening | first frame ≤ 400 ms | 281 ms | within |
| A data change | on screen ≤ 100 ms | worst 17.5 ms of 10 | within |
| Memory added, open and still | ≤ 150 MB | +107 MB | within |
| Memory added, animating (the Floor's own) | ≤ 250 MB | +94 MB at its highest; Metal's working memory on this Mac 583 MB beside it (the whole app +677 MB) | within |
| Memory once closed | no growth across repeated opens and closes | +0 MB over the last 3 of 6 more cycles; raw +67 MB once closed (the system's allocator keeps freed blocks for reuse) | within |
| Opened and closed 6 more times | the last 3 add ≤ 15 MB (no leak) | 131 → 135 → 103 → 85 → 75 → 103 MB (+0 over the last 3) | within |
| Building the scene | ≤ 50 ms | 1.69 ms on average | within |

## Every case

| Case | Seconds | CPU (% of one core) | Frames | In the first second | Drawing a frame | Footprint |
|---|---|---|---|---|---|---|
| Never opened (the app alone) | 12 | 7.021 | 0 | 0 | 0.00 ms | 55 MB |
| Open, at rest (Motion Off) | 12 | 8.523 | 1 | 0 | 3.20 ms | 162 MB |
| Animating, Calm | 12 | 8.535 | 50 | 4 | 0.65 ms | 730 MB |
| Animating, Motion Full (5 minutes) | 60 | 6.539 | 486 | 8 | 0.51 ms | 732 MB |
| Covered by another window | 10 | 5.670 | 0 | 0 | 0.00 ms | 176 MB |
| Minimised | 10 | 5.669 | 0 | 0 | 0.00 ms | 173 MB |
| Closed | 10 | 6.264 | 0 | 0 | 0.00 ms | 122 MB |

Memory while animating (Motion Full): 10 s: 732 MB, 30 s: 731 MB, 55 s: 732 MB (before opening: 55 MB).
A data change on screen, 10 tries: 2.391, 2.753, 1.646, 2.156, 1.804, 2.208, 1.512, 1.46, 17.548, 2.176 ms.

Opening, step by step (ms after it was asked for): asked 0, window 14, surface 17, settings and office 21, snapshot 22, scene 24, renderer 105, first frame 281.

After three more opens and closes: 103 MB (after the first close: 122 MB).

Memory before opening, biggest parts: TOTAL 55 MB, Malloc Small 35 MB, WebKit Malloc 4 MB, Malloc Metadata 3 MB, Stack 3 MB, __DATA_DIRTY 2 MB.
Memory open and still, biggest parts: TOTAL 162 MB, Malloc Small 61 MB, Owned physical footprint (unmapped) (graphics) 39 MB, Malloc Large 23 MB, IOSurface 8 MB, Owned physical footprint (unmapped) 6 MB.
Memory animating, biggest parts: TOTAL 732 MB, Owned physical footprint (unmapped) (graphics) 583 MB, Malloc Small 63 MB, Malloc Large 37 MB, IOSurface 12 MB, IOAccelerator (graphics) 9 MB.
Memory closed, biggest parts: TOTAL 122 MB, Malloc Small 63 MB, Malloc Large 37 MB, WebKit Malloc 4 MB, Malloc Metadata 3 MB, __DATA_DIRTY 2 MB.
Memory closed after three more opens, biggest parts: TOTAL 103 MB, Malloc Small 59 MB, Malloc Large 21 MB, WebKit Malloc 4 MB, Malloc Metadata 3 MB, Stack 2 MB.

What the window said at the end of each case: Never opened (the app alone): no answer yet; Open, at rest (Motion Off): no answer yet; Animating, Calm: seen; Animating, Motion Full (5 minutes): seen; Covered by another window: not seen; Minimised: not seen, minimised; Closed: seen.
