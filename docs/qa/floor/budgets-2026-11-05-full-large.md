# The Floor's budgets in Full resolution (D-80) on a large window, 2026-11-05

Read by `scripts/floor_harness.py` on the release build with the harness
(AJ-E3513 req. 3), on the replica company: 7 Apps, 0 sessions. Budgets: SPEC §9.7.

This run drew into a window that a window manager or a zoom had made
3440 × 1410 (4.8 times the pixels of the records beside it, close to a
Retina window of their size); the harness now sets one size for every
record. Kept as the cost of Full resolution on a large window: the setting
says it.

Machine: Mac17,15, Apple M5 Ultra, 96 GB, macOS 27.0.1.
Window: 3440 × 1410 device pixels at scale 1; drawn at those pixels, edges smoothed by a post-pass (no MSAA).
CPU is the whole app's, less the app alone; memory is its footprint (the GPU's
surfaces included) less before the Floor opened.

| Case | Budget | Measured | |
|---|---|---|---|
| Open, nothing moving | 0 frames after the first, but for data news | 2 frames in 60 s, 2 for data news | within |
| Open, nothing moving | < 0.1% of one core | 0.022% the Floor's own (the whole app +0.40% over a window alone) | within |
| Animating, Motion Full | ≤ 2.0% of one core | 2.69% the Floor's own (the whole app +0.20% over) | **over** |
| Animating, Motion Full | ≤ 1.5 ms of CPU a frame | 1.78 ms (4509 frames) | **over** |
| Animating, Calm | ≤ 2.0% of one core | 2.01% the Floor's own (671 frames) | **over** |
| Covered | 0 frames within 1 s | 0 after the first second (0 in it) | within |
| Minimised | 0 frames within 1 s | 0 after the first second (0 in it) | within |
| Closed | 0 frames | 0 | within |
| Opening | first frame ≤ 400 ms | 285 ms | within |
| A data change | on screen ≤ 100 ms | worst 13.3 ms of 10 | within |
| Memory added, open and still | ≤ 150 MB | +136 MB | within |
| Memory added, animating (the Floor's own) | ≤ 250 MB | +204 MB at its highest; Metal's working memory on this Mac 588 MB beside it (the whole app +792 MB) | within |
| Memory once closed | no growth across repeated opens and closes | +17 MB over the last 3 of 6 more cycles; raw +67 MB once closed (the system's allocator keeps freed blocks for reuse) | **over** |
| Opened and closed 6 more times | the last 3 add ≤ 15 MB (no leak) | 130 → 130 → 131 → 132 → 148 → 148 MB (+17 over the last 3) | **over** |
| Building the scene | ≤ 50 ms | 3.77 ms on average | within |

## Every case

| Case | Seconds | CPU (% of one core) | Frames | In the first second | Drawing a frame | Footprint |
|---|---|---|---|---|---|---|
| Never opened (the app alone) | 60 | 4.512 | 0 | 0 | 0.00 ms | 54 MB |
| Open, at rest (Motion Off) | 60 | 4.912 | 2 | 0 | 1.75 ms | 190 MB |
| Animating, Calm | 60 | 7.493 | 671 | 4 | 1.78 ms | 842 MB |
| Animating, Motion Full (5 minutes) | 300 | 4.713 | 4509 | 90 | 1.78 ms | 725 MB |
| Covered by another window | 30 | 1.331 | 0 | 0 | 0.00 ms | 198 MB |
| Minimised | 30 | 1.364 | 0 | 0 | 0.00 ms | 215 MB |
| Closed | 30 | 2.261 | 0 | 0 | 0.00 ms | 121 MB |

Memory while animating (Motion Full): 30 s: 846 MB, 150 s: 756 MB, 290 s: 725 MB (before opening: 54 MB).
A data change on screen, 10 tries: 8.723, 7.634, 8.177, 8.104, 7.932, 7.699, 8.111, 9.765, 13.308, 10.626 ms.

Opening, step by step (ms after it was asked for): asked 0, window 8, surface 21, settings and office 21, snapshot 22, scene 31, renderer 50, first frame 285.

After three more opens and closes: 148 MB (after the first close: 121 MB).

Memory before opening, biggest parts: TOTAL 54 MB, Malloc Small 30 MB, Malloc Large 4 MB, Malloc Metadata 3 MB, WebKit Malloc 3 MB, Stack 3 MB.
Memory open and still, biggest parts: TOTAL 190 MB, Owned physical footprint (unmapped) (graphics) 56 MB, Malloc Large 40 MB, Malloc Small 38 MB, IOAccelerator (graphics) 18 MB, Owned physical footprint (unmapped) 10 MB.
Memory animating, biggest parts: TOTAL 725 MB, Owned physical footprint (unmapped) (graphics) 588 MB, Malloc Small 43 MB, Malloc Large 40 MB, Owned physical footprint (unmapped) 13 MB, IOSurface 12 MB.
Memory closed, biggest parts: TOTAL 121 MB, Malloc Small 45 MB, Malloc Large 44 MB, IOSurface 12 MB, Malloc Metadata 3 MB, WebKit Malloc 3 MB.
Memory closed after three more opens, biggest parts: TOTAL 148 MB, Malloc Small 54 MB, Malloc Large 44 MB, IOSurface 28 MB, WebKit Malloc 4 MB, Malloc Metadata 3 MB.

What the window said at the end of each case: Never opened (the app alone): no answer yet; Open, at rest (Motion Off): no answer yet; Animating, Calm: seen; Animating, Motion Full (5 minutes): seen; Covered by another window: not seen; Minimised: not seen, minimised; Closed: seen.
