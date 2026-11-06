# The Floor's budgets, 0.4.2's second pass (materials and faces), 2026-11-06-second-first-try

Read by `scripts/floor_harness.py` on the release build with the harness
(AJ-E3513 req. 3), on the replica company: 9 Apps, 340 sessions. Budgets: SPEC §9.7.

Machine: Mac17,15, Apple M5 Ultra, 96 GB, macOS 27.0.1.
Window: 1278 × 798 device pixels at scale 1; the scene drawn at its pixel size and scaled up (no MSAA).
CPU is the whole app's, less the app alone; memory is its footprint (the GPU's
surfaces included) less before the Floor opened.

| Case | Budget | Measured | |
|---|---|---|---|
| Open, nothing moving | 0 frames after the first, but for data news | 1 frames in 60 s, 1 for data news | within |
| Open, nothing moving | < 0.1% of one core | 0.047% the Floor's own (the whole app -0.25% over a window alone) | within |
| Animating, Motion Full | ≤ 2.0% of one core | 1.41% the Floor's own (the whole app +1.84% over) | within |
| Animating, Motion Full | ≤ 1.5 ms of CPU a frame | 1.20 ms (3439 frames) | within |
| Animating, Calm | ≤ 2.0% of one core | 0.26% the Floor's own (242 frames) | within |
| Covered | 0 frames within 1 s | 0 after the first second (0 in it) | within |
| Minimised | 0 frames within 1 s | 0 after the first second (0 in it) | within |
| Closed | 0 frames | 0 | within |
| Opening | first frame ≤ 400 ms | 291 ms | within |
| A data change | on screen ≤ 100 ms | worst 24.1 ms of 10 | within |
| Memory added, open and still | ≤ 150 MB | +118 MB | within |
| Memory added, animating (the Floor's own) | ≤ 250 MB | +147 MB at its highest; Metal's working memory on this Mac 576 MB beside it (the whole app +723 MB) | within |
| Memory once closed | no growth across repeated opens and closes | +23 MB over the last 3 of 6 more cycles; raw +93 MB once closed (the system's allocator keeps freed blocks for reuse) | **over** |
| Opened and closed 6 more times | the last 3 add ≤ 15 MB (no leak) | 159 → 169 → 178 → 184 → 186 → 201 MB (+23 over the last 3) | **over** |
| Building the scene | ≤ 50 ms | 9.16 ms on average | within |

## Every case

| Case | Seconds | CPU (% of one core) | Frames | In the first second | Drawing a frame | Footprint |
|---|---|---|---|---|---|---|
| Never opened (the app alone) | 60 | 2.564 | 0 | 0 | 0.00 ms | 64 MB |
| Open, at rest (Motion Off) | 60 | 2.314 | 1 | 0 | 2.17 ms | 182 MB |
| Animating, Calm | 60 | 3.098 | 242 | 4 | 0.53 ms | 746 MB |
| Animating, Motion Full (5 minutes) | 300 | 4.400 | 3439 | 8 | 1.20 ms | 756 MB |
| Covered by another window | 30 | 2.694 | 0 | 0 | 0.00 ms | 201 MB |
| Minimised | 30 | 2.362 | 0 | 0 | 0.00 ms | 201 MB |
| Closed | 30 | 1.988 | 0 | 0 | 0.00 ms | 157 MB |

Memory while animating (Motion Full): 30 s: 747 MB, 150 s: 787 MB, 290 s: 756 MB (before opening: 64 MB).
A data change on screen, 10 tries: 21.739, 9.501, 24.12, 23.474, 21.149, 23.243, 16.927, 14.129, 9.251, 10.684 ms.

Opening, step by step (ms after it was asked for): asked 0, window 6, surface 18, settings and office 18, snapshot 22, scene 33, renderer 62, first frame 291.

After three more opens and closes: 132 MB (after the first close: 157 MB).

Memory before opening, biggest parts: TOTAL 64 MB, Malloc Small 41 MB, Malloc Large 4 MB, WebKit Malloc 4 MB, Malloc Metadata 3 MB, __DATA_DIRTY 2 MB.
Memory open and still, biggest parts: TOTAL 182 MB, Malloc Small 60 MB, Owned physical footprint (unmapped) (graphics) 45 MB, Malloc Large 37 MB, IOAccelerator (graphics) 10 MB, IOSurface 8 MB.
Memory animating, biggest parts: TOTAL 756 MB, Owned physical footprint (unmapped) (graphics) 576 MB, Malloc Small 71 MB, Malloc Large 45 MB, Owned physical footprint (unmapped) 21 MB, IOSurface 12 MB.
Memory closed, biggest parts: TOTAL 157 MB, Malloc Small 76 MB, Malloc Large 45 MB, IOSurface 12 MB, WebKit Malloc 4 MB, Malloc Metadata 3 MB.
Memory closed after three more opens, biggest parts: TOTAL 132 MB, Malloc Small 59 MB, IOSurface 40 MB, Malloc Large 8 MB, WebKit Malloc 4 MB, Malloc Metadata 3 MB.

What the window said at the end of each case: Never opened (the app alone): no answer yet; Open, at rest (Motion Off): no answer yet; Animating, Calm: seen; Animating, Motion Full (5 minutes): seen; Covered by another window: not seen; Minimised: not seen, minimised; Closed: seen.
