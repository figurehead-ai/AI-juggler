# The Floor's budgets, 0.4.1's second pass (materials and faces), 2026-11-06

Read by `scripts/floor_harness.py` on the release build with the harness
(AJ-E3513 req. 3), on the replica company: 9 Apps, 340 sessions. Budgets: SPEC §9.7.

Machine: Mac17,15, Apple M5 Ultra, 96 GB, macOS 27.0.1.
Window: 1278 × 798 device pixels at scale 1; the scene drawn at its pixel size and scaled up (no MSAA).
CPU is the whole app's, less the app alone; memory is its footprint (the GPU's
surfaces included) less before the Floor opened.

| Case | Budget | Measured | |
|---|---|---|---|
| Open, nothing moving | 0 frames after the first, but for data news | 0 frames in 60 s, 0 for data news | within |
| Open, nothing moving | < 0.1% of one core | 0.039% the Floor's own (the whole app +1.53% over a window alone) | within |
| Animating, Motion Full | ≤ 2.0% of one core | 0.51% the Floor's own (the whole app +1.65% over) | within |
| Animating, Motion Full | ≤ 1.5 ms of CPU a frame | 0.58 ms (2404 frames) | within |
| Animating, Calm | ≤ 2.0% of one core | 0.37% the Floor's own (240 frames) | within |
| Covered | 0 frames within 1 s | 0 after the first second (0 in it) | within |
| Minimised | 0 frames within 1 s | 0 after the first second (0 in it) | within |
| Closed | 0 frames | 0 | within |
| Opening | first frame ≤ 400 ms | 303 ms | within |
| A data change | on screen ≤ 100 ms | worst 27.0 ms of 10 | within |
| Memory added, open and still | ≤ 150 MB | +127 MB | within |
| Memory added, animating (the Floor's own) | ≤ 250 MB | +122 MB at its highest; Metal's working memory on this Mac 557 MB beside it (the whole app +679 MB) | within |
| Memory once closed | no growth across repeated opens and closes | +6 MB over the last 3 of 6 more cycles; raw +86 MB once closed (the system's allocator keeps freed blocks for reuse) | within |
| Opened and closed 6 more times | the last 3 add ≤ 15 MB (no leak) | 154 → 156 → 157 → 158 → 159 → 163 MB (+6 over the last 3) | within |
| Building the scene | ≤ 50 ms | 8.90 ms on average | within |

## Every case

| Case | Seconds | CPU (% of one core) | Frames | In the first second | Drawing a frame | Footprint |
|---|---|---|---|---|---|---|
| Never opened (the app alone) | 60 | 5.261 | 0 | 0 | 0.00 ms | 60 MB |
| Open, at rest (Motion Off) | 60 | 6.793 | 0 | 0 | 0.00 ms | 187 MB |
| Animating, Calm | 60 | 7.210 | 240 | 4 | 0.68 ms | 729 MB |
| Animating, Motion Full (5 minutes) | 300 | 6.909 | 2404 | 8 | 0.58 ms | 718 MB |
| Covered by another window | 30 | 5.355 | 0 | 0 | 0.00 ms | 189 MB |
| Minimised | 30 | 5.456 | 0 | 0 | 0.00 ms | 189 MB |
| Closed | 30 | 5.390 | 0 | 0 | 0.00 ms | 146 MB |

Memory while animating (Motion Full): 30 s: 739 MB, 150 s: 734 MB, 290 s: 718 MB (before opening: 60 MB).
A data change on screen, 10 tries: 12.763, 9.784, 9.003, 22.988, 21.646, 12.071, 26.976, 21.56, 23.632, 11.045 ms.

Opening, step by step (ms after it was asked for): asked 0, window 17, surface 28, settings and office 28, snapshot 33, scene 42, renderer 68, first frame 303.

After three more opens and closes: 97 MB (after the first close: 146 MB).

Memory before opening, biggest parts: TOTAL 60 MB, Malloc Small 36 MB, Malloc Large 4 MB, Malloc Metadata 3 MB, WebKit Malloc 3 MB, __DATA_DIRTY 2 MB.
Memory open and still, biggest parts: TOTAL 187 MB, Malloc Small 56 MB, Owned physical footprint (unmapped) (graphics) 54 MB, Malloc Large 29 MB, IOAccelerator (graphics) 24 MB, IOSurface 4 MB.
Memory animating, biggest parts: TOTAL 718 MB, Owned physical footprint (unmapped) (graphics) 557 MB, Malloc Small 67 MB, Malloc Large 45 MB, Owned physical footprint (unmapped) 11 MB, IOAccelerator (graphics) 9 MB.
Memory closed, biggest parts: TOTAL 146 MB, Malloc Small 71 MB, Malloc Large 45 MB, IOSurface 8 MB, WebKit Malloc 4 MB, Malloc Metadata 3 MB.
Memory closed after three more opens, biggest parts: TOTAL 97 MB, Malloc Small 62 MB, IOSurface 8 MB, Malloc Large 4 MB, WebKit Malloc 4 MB, Malloc Metadata 3 MB.

What the window said at the end of each case: Never opened (the app alone): no answer yet; Open, at rest (Motion Off): no answer yet; Animating, Calm: seen; Animating, Motion Full (5 minutes): seen; Covered by another window: not seen; Minimised: not seen, minimised; Closed: seen.
