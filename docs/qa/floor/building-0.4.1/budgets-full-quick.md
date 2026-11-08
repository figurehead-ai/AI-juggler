# The Floor's budgets in Full resolution (D-80), 0.4.1's second pass (materials and faces), the building view (AJ-E3516's cross-section), 2026-11-08

Read by `scripts/floor_harness.py` on the release build with the harness
(AJ-E3513 req. 3), on the replica company: 0 Apps, 0 sessions. Budgets: SPEC §9.7.

Machine: Mac17,15, Apple M5 Ultra, 96 GB, macOS 27.0.1.
Window: 1278 × 798 device pixels at scale 1; drawn at those pixels, edges smoothed by a post-pass (no MSAA).
CPU is the whole app's, less the app alone; memory is its footprint (the GPU's
surfaces included) less before the Floor opened.

| Case | Budget | Measured | |
|---|---|---|---|
| Open, nothing moving | 0 frames after the first, but for data news | 2 frames in 12 s, 2 for data news | within |
| Open, nothing moving | < 0.1% of one core | 0.046% the Floor's own (the whole app +1.07% over a window alone) | within |
| Animating, Motion Full | ≤ 2.0% of one core | 0.40% the Floor's own (the whole app +1.32% over) | within |
| Animating, Motion Full | ≤ 1.5 ms of CPU a frame | 0.47 ms (483 frames) | within |
| Animating, Calm | ≤ 2.0% of one core | 0.22% the Floor's own (48 frames) | within |
| Covered | 0 frames within 1 s | 0 after the first second (0 in it) | within |
| Minimised | 0 frames within 1 s | 0 after the first second (0 in it) | within |
| Closed | 0 frames | 0 | within |
| Opening | first frame ≤ 400 ms | 289 ms | within |
| A data change | on screen ≤ 100 ms | worst 3.2 ms of 10 | within |
| Memory added, open and still | ≤ 150 MB | +99 MB | within |
| Memory added, animating (the Floor's own) | ≤ 250 MB | +83 MB at its highest; Metal's working memory on this Mac 592 MB beside it (the whole app +675 MB) | within |
| Memory once closed | no growth across repeated opens and closes | +4 MB over the last 3 of 6 more cycles; raw +55 MB once closed (the system's allocator keeps freed blocks for reuse) | within |
| Opened and closed 6 more times | the last 3 add ≤ 15 MB (no leak) | 127 → 135 → 70 → 102 → 91 → 74 MB (+4 over the last 3) | within |
| Building the scene | ≤ 50 ms | 2.33 ms on average | within |

## Every case

| Case | Seconds | CPU (% of one core) | Frames | In the first second | Drawing a frame | Footprint |
|---|---|---|---|---|---|---|
| Never opened (the app alone) | 12 | 5.225 | 0 | 0 | 0.00 ms | 50 MB |
| Open, at rest (Motion Off) | 12 | 6.291 | 2 | 0 | 0.60 ms | 149 MB |
| Animating, Calm | 12 | 6.802 | 48 | 4 | 0.48 ms | 708 MB |
| Animating, Motion Full (5 minutes) | 60 | 6.541 | 483 | 8 | 0.47 ms | 725 MB |
| Covered by another window | 10 | 5.567 | 0 | 0 | 0.00 ms | 165 MB |
| Minimised | 10 | 5.070 | 0 | 0 | 0.00 ms | 169 MB |
| Closed | 10 | 4.771 | 0 | 0 | 0.00 ms | 105 MB |

Memory while animating (Motion Full): 10 s: 724 MB, 30 s: 724 MB, 55 s: 725 MB (before opening: 50 MB).
A data change on screen, 10 tries: 3.18, 1.774, 1.741, 2.338, 2.087, 1.615, 1.476, 1.442, 1.546, 3.084 ms.

Opening, step by step (ms after it was asked for): asked 0, window 15, surface 26, settings and office 26, snapshot 27, scene 28, renderer 103, first frame 289.

After three more opens and closes: 74 MB (after the first close: 105 MB).

Memory before opening, biggest parts: TOTAL 50 MB, Malloc Small 31 MB, Malloc Metadata 3 MB, WebKit Malloc 3 MB, __DATA_DIRTY 2 MB, Stack 2 MB.
Memory open and still, biggest parts: TOTAL 149 MB, Owned physical footprint (unmapped) (graphics) 48 MB, Malloc Small 47 MB, Malloc Large 18 MB, Owned physical footprint (unmapped) 8 MB, IOSurface 4 MB.
Memory animating, biggest parts: TOTAL 725 MB, Owned physical footprint (unmapped) (graphics) 592 MB, Malloc Small 51 MB, Malloc Large 32 MB, IOSurface 12 MB, IOAccelerator (graphics) 9 MB.
Memory closed, biggest parts: TOTAL 105 MB, Malloc Small 52 MB, Malloc Large 32 MB, WebKit Malloc 4 MB, Malloc Metadata 3 MB, __DATA_DIRTY 2 MB.
Memory closed after three more opens, biggest parts: TOTAL 74 MB, Malloc Small 52 MB, WebKit Malloc 4 MB, Malloc Metadata 3 MB, Stack 2 MB, __DATA_DIRTY 2 MB.

What the window said at the end of each case: Never opened (the app alone): no answer yet; Open, at rest (Motion Off): no answer yet; Animating, Calm: seen; Animating, Motion Full (5 minutes): seen; Covered by another window: not seen; Minimised: not seen, minimised; Closed: seen.
