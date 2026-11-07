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
| Open, nothing moving | < 0.1% of one core | 0.042% the Floor's own (the whole app +0.43% over a window alone) | within |
| Animating, Motion Full | ≤ 2.0% of one core | 0.51% the Floor's own (the whole app +0.41% over) | within |
| Animating, Motion Full | ≤ 1.5 ms of CPU a frame | 0.56 ms (2407 frames) | within |
| Animating, Calm | ≤ 2.0% of one core | 0.04% the Floor's own (0 frames) | within |
| Covered | 0 frames within 1 s | 0 after the first second (0 in it) | within |
| Minimised | 0 frames within 1 s | 0 after the first second (0 in it) | within |
| Closed | 0 frames | 0 | within |
| Opening | first frame ≤ 400 ms | 513 ms | **over** |
| A data change | on screen ≤ 100 ms | worst 22.1 ms of 10 | within |
| Memory added, open and still | ≤ 150 MB | +100 MB | within |
| Memory added, animating (the Floor's own) | ≤ 250 MB | +104 MB at its highest; Metal's working memory on this Mac 573 MB beside it (the whole app +677 MB) | within |
| Memory once closed | no growth across repeated opens and closes | +41 MB over the last 3 of 6 more cycles; raw +77 MB once closed (the system's allocator keeps freed blocks for reuse) | **over** |
| Opened and closed 6 more times | the last 3 add ≤ 15 MB (no leak) | 156 → 177 → 189 → 205 → 218 → 230 MB (+41 over the last 3) | **over** |
| Building the scene | ≤ 50 ms | 6.78 ms on average | within |

## Every case

| Case | Seconds | CPU (% of one core) | Frames | In the first second | Drawing a frame | Footprint |
|---|---|---|---|---|---|---|
| Never opened (the app alone) | 60 | 7.857 | 0 | 0 | 0.00 ms | 60 MB |
| Open, at rest (Motion Off) | 60 | 8.289 | 0 | 0 | 0.00 ms | 160 MB |
| Animating, Calm | 60 | 7.989 | 0 | 0 | 0.00 ms | 177 MB |
| Animating, Motion Full (5 minutes) | 300 | 8.267 | 2407 | 8 | 0.56 ms | 723 MB |
| Covered by another window | 30 | 4.725 | 0 | 0 | 0.00 ms | 180 MB |
| Minimised | 30 | 5.488 | 0 | 0 | 0.00 ms | 180 MB |
| Closed | 30 | 7.876 | 0 | 0 | 0.00 ms | 137 MB |

Memory while animating (Motion Full): 30 s: 737 MB, 150 s: 737 MB, 290 s: 723 MB (before opening: 60 MB).
A data change on screen, 10 tries: 11.23, 9.981, 9.965, 12.27, 20.676, 18.778, 19.582, 20.3, 16.759, 22.071 ms.

Opening, step by step (ms after it was asked for): asked 0, window 11, surface 22, settings and office 25, snapshot 28, scene 40, renderer 501, first frame 513.

After three more opens and closes: 170 MB (after the first close: 137 MB).

Memory before opening, biggest parts: TOTAL 60 MB, Malloc Small 36 MB, Malloc Large 4 MB, Malloc Metadata 3 MB, WebKit Malloc 3 MB, Stack 2 MB.
Memory open and still, biggest parts: TOTAL 160 MB, Malloc Small 56 MB, Owned physical footprint (unmapped) (graphics) 45 MB, Malloc Large 29 MB, IOSurface 8 MB, Malloc Metadata 3 MB.
Memory animating, biggest parts: TOTAL 723 MB, Owned physical footprint (unmapped) (graphics) 573 MB, Malloc Small 59 MB, Malloc Large 45 MB, Owned physical footprint (unmapped) 9 MB, IOAccelerator (graphics) 9 MB.
Memory closed, biggest parts: TOTAL 137 MB, Malloc Small 62 MB, Malloc Large 45 MB, IOSurface 8 MB, Malloc Metadata 3 MB, WebKit Malloc 3 MB.
Memory closed after three more opens, biggest parts: TOTAL 170 MB, IOSurface 79 MB, Malloc Small 54 MB, Malloc Large 13 MB, Malloc Metadata 3 MB, WebKit Malloc 3 MB.

What the window said at the end of each case: Never opened (the app alone): no answer yet; Open, at rest (Motion Off): no answer yet; Animating, Calm: no answer yet; Animating, Motion Full (5 minutes): seen; Covered by another window: not seen; Minimised: not seen, minimised; Closed: seen.
