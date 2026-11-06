# The Floor's budgets in Full resolution (D-80), 0.4.2's second pass (materials and faces), 2026-11-06

Read by `scripts/floor_harness.py` on the release build with the harness
(AJ-E3513 req. 3), on the replica company: 9 Apps, 340 sessions. Budgets: SPEC §9.7.

Machine: Mac17,15, Apple M5 Ultra, 96 GB, macOS 27.0.1.
Window: 1278 × 798 device pixels at scale 1; drawn at those pixels, edges smoothed by a post-pass (no MSAA).
CPU is the whole app's, less the app alone; memory is its footprint (the GPU's
surfaces included) less before the Floor opened.

| Case | Budget | Measured | |
|---|---|---|---|
| Open, nothing moving | 0 frames after the first, but for data news | 0 frames in 60 s, 0 for data news | within |
| Open, nothing moving | < 0.1% of one core | 0.031% the Floor's own (the whole app -1.70% over a window alone) | within |
| Animating, Motion Full | ≤ 2.0% of one core | 0.57% the Floor's own (the whole app +0.61% over) | within |
| Animating, Motion Full | ≤ 1.5 ms of CPU a frame | 0.60 ms (2412 frames) | within |
| Animating, Calm | ≤ 2.0% of one core | 0.26% the Floor's own (240 frames) | within |
| Covered | 0 frames within 1 s | 0 after the first second (0 in it) | within |
| Minimised | 0 frames within 1 s | 0 after the first second (0 in it) | within |
| Closed | 0 frames | 0 | within |
| Opening | first frame ≤ 400 ms | 293 ms | within |
| A data change | on screen ≤ 100 ms | worst 12.1 ms of 10 | within |
| Memory added, open and still | ≤ 150 MB | +115 MB | within |
| Memory added, animating (the Floor's own) | ≤ 250 MB | +85 MB at its highest; Metal's working memory on this Mac 598 MB beside it (the whole app +683 MB) | within |
| Memory once closed | no growth across repeated opens and closes | +22 MB over the last 3 of 6 more cycles; raw +24 MB once closed (the system's allocator keeps freed blocks for reuse) | **over** |
| Opened and closed 6 more times | the last 3 add ≤ 15 MB (no leak) | 94 → 106 → 127 → 141 → 144 → 149 MB (+22 over the last 3) | **over** |
| Building the scene | ≤ 50 ms | 6.57 ms on average | within |

## Every case

| Case | Seconds | CPU (% of one core) | Frames | In the first second | Drawing a frame | Footprint |
|---|---|---|---|---|---|---|
| Never opened (the app alone) | 60 | 7.142 | 0 | 0 | 0.00 ms | 61 MB |
| Open, at rest (Motion Off) | 60 | 5.445 | 0 | 0 | 0.00 ms | 176 MB |
| Animating, Calm | 60 | 6.177 | 240 | 4 | 0.50 ms | 705 MB |
| Animating, Motion Full (5 minutes) | 300 | 7.757 | 2412 | 8 | 0.60 ms | 721 MB |
| Covered by another window | 30 | 7.318 | 0 | 0 | 0.00 ms | 163 MB |
| Minimised | 30 | 4.924 | 0 | 0 | 0.00 ms | 160 MB |
| Closed | 30 | 6.053 | 0 | 0 | 0.00 ms | 85 MB |

Memory while animating (Motion Full): 30 s: 715 MB, 150 s: 706 MB, 290 s: 744 MB (before opening: 61 MB).
A data change on screen, 10 tries: 12.131, 10.918, 10.585, 10.814, 11.182, 10.656, 10.036, 10.452, 9.275, 10.502 ms.

Opening, step by step (ms after it was asked for): asked 0, window 6, surface 18, settings and office 18, snapshot 22, scene 48, renderer 79, first frame 293.

After three more opens and closes: 148 MB (after the first close: 85 MB).

Memory before opening, biggest parts: TOTAL 61 MB, Malloc Small 37 MB, Malloc Large 4 MB, WebKit Malloc 4 MB, Malloc Metadata 3 MB, Stack 2 MB.
Memory open and still, biggest parts: TOTAL 176 MB, Owned physical footprint (unmapped) (graphics) 63 MB, Malloc Small 48 MB, IOAccelerator (graphics) 24 MB, Malloc Large 16 MB, WebKit Malloc 4 MB.
Memory animating, biggest parts: TOTAL 721 MB, Owned physical footprint (unmapped) (graphics) 598 MB, Malloc Small 43 MB, Malloc Large 24 MB, Owned physical footprint (unmapped) 14 MB, IOSurface 12 MB.
Memory closed, biggest parts: TOTAL 85 MB, Malloc Small 45 MB, IOSurface 12 MB, Malloc Large 4 MB, WebKit Malloc 4 MB, Malloc Metadata 3 MB.
Memory closed after three more opens, biggest parts: TOTAL 148 MB, IOSurface 60 MB, Malloc Small 44 MB, Malloc Large 20 MB, WebKit Malloc 4 MB, Malloc Metadata 3 MB.

What the window said at the end of each case: Never opened (the app alone): no answer yet; Open, at rest (Motion Off): no answer yet; Animating, Calm: seen; Animating, Motion Full (5 minutes): seen; Covered by another window: not seen; Minimised: not seen, minimised; Closed: seen.
