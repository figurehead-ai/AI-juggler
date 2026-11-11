# The Floor's budgets in Full resolution (D-80), 0.4.1's second pass (materials and faces), 2026-11-06-full-second-first-try

Read by `scripts/floor_harness.py` on the release build with the harness
(AJ-E3513 req. 3), on the replica company: 9 Apps, 340 sessions. Budgets: SPEC §9.7.

Machine: Mac17,15, Apple M5 Ultra, 96 GB, macOS 27.0.1.
Window: 1278 × 798 device pixels at scale 1; drawn at those pixels, edges smoothed by a post-pass (no MSAA).
CPU is the whole app's, less the app alone; memory is its footprint (the GPU's
surfaces included) less before the Floor opened.

| Case | Budget | Measured | |
|---|---|---|---|
| Open, nothing moving | 0 frames after the first, but for data news | 0 frames in 60 s, 0 for data news | within |
| Open, nothing moving | < 0.1% of one core | 0.122% the Floor's own (the whole app +2.23% over a window alone) | **over** |
| Animating, Motion Full | ≤ 2.0% of one core | 0.50% the Floor's own (the whole app +2.42% over) | within |
| Animating, Motion Full | ≤ 1.5 ms of CPU a frame | 0.57 ms (2409 frames) | within |
| Animating, Calm | ≤ 2.0% of one core | 0.23% the Floor's own (240 frames) | within |
| Covered | 0 frames within 1 s | 0 after the first second (0 in it) | within |
| Minimised | 0 frames within 1 s | 0 after the first second (0 in it) | within |
| Closed | 0 frames | 0 | within |
| Opening | first frame ≤ 400 ms | 302 ms | within |
| A data change | on screen ≤ 100 ms | worst 11.3 ms of 10 | within |
| Memory added, open and still | ≤ 150 MB | +134 MB | within |
| Memory added, animating (the Floor's own) | ≤ 250 MB | +99 MB at its highest; Metal's working memory on this Mac 601 MB beside it (the whole app +700 MB) | within |
| Memory once closed | no growth across repeated opens and closes | +4 MB over the last 3 of 6 more cycles; raw +77 MB once closed (the system's allocator keeps freed blocks for reuse) | within |
| Opened and closed 6 more times | the last 3 add ≤ 15 MB (no leak) | 142 → 144 → 147 → 148 → 149 → 151 MB (+4 over the last 3) | within |
| Building the scene | ≤ 50 ms | 6.07 ms on average | within |

## Every case

| Case | Seconds | CPU (% of one core) | Frames | In the first second | Drawing a frame | Footprint |
|---|---|---|---|---|---|---|
| Never opened (the app alone) | 60 | 5.344 | 0 | 0 | 0.00 ms | 60 MB |
| Open, at rest (Motion Off) | 60 | 7.576 | 0 | 0 | 0.00 ms | 194 MB |
| Animating, Calm | 60 | 6.327 | 240 | 4 | 0.53 ms | 737 MB |
| Animating, Motion Full (5 minutes) | 300 | 7.767 | 2409 | 8 | 0.57 ms | 760 MB |
| Covered by another window | 30 | 6.019 | 0 | 0 | 0.00 ms | 202 MB |
| Minimised | 30 | 6.553 | 0 | 0 | 0.00 ms | 193 MB |
| Closed | 30 | 5.820 | 0 | 0 | 0.00 ms | 137 MB |

Memory while animating (Motion Full): 30 s: 749 MB, 150 s: 750 MB, 290 s: 760 MB (before opening: 60 MB).
A data change on screen, 10 tries: 10.846, 9.627, 10.759, 10.06, 10.696, 9.192, 9.95, 9.603, 11.325, 11.308 ms.

Opening, step by step (ms after it was asked for): asked 0, window 17, surface 30, settings and office 30, snapshot 34, scene 44, renderer 72, first frame 302.

After three more opens and closes: 151 MB (after the first close: 137 MB).

Memory before opening, biggest parts: TOTAL 60 MB, Malloc Small 37 MB, Malloc Large 4 MB, Malloc Metadata 3 MB, WebKit Malloc 3 MB, Stack 2 MB.
Memory open and still, biggest parts: TOTAL 194 MB, Owned physical footprint (unmapped) (graphics) 63 MB, Malloc Small 54 MB, Malloc Large 29 MB, IOAccelerator (graphics) 24 MB, IOSurface 4 MB.
Memory animating, biggest parts: TOTAL 760 MB, Owned physical footprint (unmapped) (graphics) 601 MB, Malloc Small 56 MB, Malloc Large 45 MB, Owned physical footprint (unmapped) 17 MB, IOSurface 12 MB.
Memory closed, biggest parts: TOTAL 137 MB, Malloc Small 58 MB, Malloc Large 45 MB, IOSurface 12 MB, Malloc Metadata 3 MB, WebKit Malloc 3 MB.
Memory closed after three more opens, biggest parts: TOTAL 151 MB, Malloc Small 71 MB, Malloc Large 45 MB, IOSurface 12 MB, Malloc Metadata 3 MB, WebKit Malloc 3 MB.

What the window said at the end of each case: Never opened (the app alone): no answer yet; Open, at rest (Motion Off): no answer yet; Animating, Calm: seen; Animating, Motion Full (5 minutes): seen; Covered by another window: not seen; Minimised: not seen, minimised; Closed: seen.
