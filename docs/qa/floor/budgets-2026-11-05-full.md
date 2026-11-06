# The Floor's budgets in Full resolution (D-80), 2026-11-05

Read by `scripts/floor_harness.py` on the release build with the harness
(AJ-E3513 req. 3), on the replica company: 7 Apps, 0 sessions. Budgets: SPEC §9.7.

Machine: Mac17,15, Apple M5 Ultra, 96 GB, macOS 27.0.1.
Window: 1278 × 798 device pixels at scale 1; drawn at those pixels, edges smoothed by a post-pass (no MSAA).
CPU is the whole app's, less the app alone; memory is its footprint (the GPU's
surfaces included) less before the Floor opened.

| Case | Budget | Measured | |
|---|---|---|---|
| Open, nothing moving | 0 frames after the first, but for data news | 0 frames in 60 s, 0 for data news | within |
| Open, nothing moving | < 0.1% of one core | 0.005% the Floor's own (the whole app +0.78% over a window alone) | within |
| Animating, Motion Full | ≤ 2.0% of one core | 0.57% the Floor's own (the whole app +2.08% over) | within |
| Animating, Motion Full | ≤ 1.5 ms of CPU a frame | 0.70 ms (2405 frames) | within |
| Animating, Calm | ≤ 2.0% of one core | 0.36% the Floor's own (240 frames) | within |
| Covered | 0 frames within 1 s | 0 after the first second (0 in it) | within |
| Minimised | 0 frames within 1 s | 0 after the first second (0 in it) | within |
| Closed | 0 frames | 0 | within |
| Opening | first frame ≤ 400 ms | 297 ms | within |
| A data change | on screen ≤ 100 ms | worst 17.2 ms of 10 | within |
| Memory added, open and still | ≤ 150 MB | +154 MB | **over** |
| Memory added, animating (the Floor's own) | ≤ 250 MB | +78 MB at its highest; Metal's working memory on this Mac 584 MB beside it (the whole app +662 MB) | within |
| Memory once closed | no growth across repeated opens and closes | +2 MB over the last 3 of 6 more cycles; raw +68 MB once closed (the system's allocator keeps freed blocks for reuse) | within |
| Opened and closed 6 more times | the last 3 add ≤ 15 MB (no leak) | 124 → 124 → 125 → 125 → 126 → 127 MB (+2 over the last 3) | within |
| Building the scene | ≤ 50 ms | 5.11 ms on average | within |

## Every case

| Case | Seconds | CPU (% of one core) | Frames | In the first second | Drawing a frame | Footprint |
|---|---|---|---|---|---|---|
| Never opened (the app alone) | 60 | 5.229 | 0 | 0 | 0.00 ms | 56 MB |
| Open, at rest (Motion Off) | 60 | 6.011 | 0 | 0 | 0.00 ms | 210 MB |
| Animating, Calm | 60 | 6.774 | 240 | 4 | 0.69 ms | 710 MB |
| Animating, Motion Full (5 minutes) | 300 | 7.310 | 2405 | 8 | 0.70 ms | 718 MB |
| Covered by another window | 30 | 6.052 | 0 | 0 | 0.00 ms | 178 MB |
| Minimised | 30 | 5.622 | 0 | 0 | 0.00 ms | 178 MB |
| Closed | 30 | 5.884 | 0 | 0 | 0.00 ms | 124 MB |

Memory while animating (Motion Full): 30 s: 712 MB, 150 s: 681 MB, 290 s: 718 MB (before opening: 56 MB).
A data change on screen, 10 tries: 17.172, 10.49, 11.899, 12.563, 12.771, 10.504, 12.216, 11.182, 11.99, 13.729 ms.

Opening, step by step (ms after it was asked for): asked 0, window 14, surface 16, settings and office 17, snapshot 18, scene 27, renderer 46, first frame 297.

After three more opens and closes: 127 MB (after the first close: 124 MB).

Memory before opening, biggest parts: TOTAL 56 MB, Malloc Small 33 MB, Malloc Large 4 MB, WebKit Malloc 3 MB, Malloc Metadata 3 MB, Stack 3 MB.
Memory open and still, biggest parts: TOTAL 210 MB, Owned physical footprint (unmapped) (graphics) 73 MB, Malloc Large 40 MB, Malloc Small 38 MB, IOAccelerator (graphics) 35 MB, IOSurface 4 MB.
Memory animating, biggest parts: TOTAL 718 MB, Owned physical footprint (unmapped) (graphics) 584 MB, Malloc Small 43 MB, Malloc Large 40 MB, IOSurface 12 MB, Owned physical footprint (unmapped) 10 MB.
Memory closed, biggest parts: TOTAL 124 MB, Malloc Small 47 MB, Malloc Large 44 MB, IOSurface 12 MB, Malloc Metadata 3 MB, WebKit Malloc 3 MB.
Memory closed after three more opens, biggest parts: TOTAL 127 MB, Malloc Small 49 MB, Malloc Large 44 MB, IOSurface 12 MB, Malloc Metadata 3 MB, WebKit Malloc 3 MB.

What the window said at the end of each case: Never opened (the app alone): no answer yet; Open, at rest (Motion Off): no answer yet; Animating, Calm: seen; Animating, Motion Full (5 minutes): seen; Covered by another window: not seen; Minimised: not seen, minimised; Closed: seen.
