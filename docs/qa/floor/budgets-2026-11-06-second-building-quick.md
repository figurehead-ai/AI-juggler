# The Floor's budgets, 0.4.1's second pass (materials and faces), the building view (AJ-E3516's cross-section), 2026-11-06

Read by `scripts/floor_harness.py` on the release build with the harness
(AJ-E3513 req. 3), on the replica company: 9 Apps, 340 sessions. Budgets: SPEC §9.7.

Machine: Mac17,15, Apple M5 Ultra, 96 GB, macOS 27.0.1.
Window: 1278 × 798 device pixels at scale 1; the scene drawn at its pixel size and scaled up (no MSAA).
CPU is the whole app's, less the app alone; memory is its footprint (the GPU's
surfaces included) less before the Floor opened.

| Case | Budget | Measured | |
|---|---|---|---|
| Open, nothing moving | 0 frames after the first, but for data news | 0 frames in 12 s, 0 for data news | within |
| Open, nothing moving | < 0.1% of one core | 0.000% the Floor's own (the whole app +0.02% over a window alone) | within |
| Animating, Motion Full | ≤ 2.0% of one core | 0.46% the Floor's own (the whole app +0.10% over) | within |
| Animating, Motion Full | ≤ 1.5 ms of CPU a frame | 0.53 ms (482 frames) | within |
| Animating, Calm | ≤ 2.0% of one core | 0.08% the Floor's own (0 frames) | within |
| Covered | 0 frames within 1 s | 0 after the first second (0 in it) | within |
| Minimised | 0 frames within 1 s | 0 after the first second (0 in it) | within |
| Closed | 0 frames | 0 | within |
| Opening | first frame ≤ 400 ms | 299 ms | within |
| A data change | on screen ≤ 100 ms | worst 30.1 ms of 10 | within |
| Memory added, open and still | ≤ 150 MB | +99 MB | within |
| Memory added, animating (the Floor's own) | ≤ 250 MB | +86 MB at its highest; Metal's working memory on this Mac 589 MB beside it (the whole app +675 MB) | within |
| Memory once closed | no growth across repeated opens and closes | +38 MB over the last 3 of 6 more cycles; raw +79 MB once closed (the system's allocator keeps freed blocks for reuse) | **over** |
| Opened and closed 6 more times | the last 3 add ≤ 15 MB (no leak) | 162 → 175 → 193 → 206 → 218 → 231 MB (+38 over the last 3) | **over** |
| Building the scene | ≤ 50 ms | 14.35 ms on average | within |

## Every case

| Case | Seconds | CPU (% of one core) | Frames | In the first second | Drawing a frame | Footprint |
|---|---|---|---|---|---|---|
| Never opened (the app alone) | 12 | 7.618 | 0 | 0 | 0.00 ms | 65 MB |
| Open, at rest (Motion Off) | 12 | 7.636 | 0 | 0 | 0.00 ms | 164 MB |
| Animating, Calm | 12 | 7.051 | 0 | 0 | 0.00 ms | 180 MB |
| Animating, Motion Full (5 minutes) | 60 | 7.722 | 482 | 8 | 0.53 ms | 740 MB |
| Covered by another window | 10 | 8.829 | 0 | 0 | 0.00 ms | 181 MB |
| Minimised | 10 | 7.539 | 0 | 0 | 0.00 ms | 181 MB |
| Closed | 10 | 8.462 | 0 | 0 | 0.00 ms | 144 MB |

Memory while animating (Motion Full): 10 s: 739 MB, 30 s: 739 MB, 55 s: 739 MB (before opening: 65 MB).
A data change on screen, 10 tries: 26.554, 26.568, 26.856, 9.694, 25.983, 27.211, 29.794, 30.083, 27.277, 23.532 ms.

Opening, step by step (ms after it was asked for): asked 0, window 5, surface 20, settings and office 21, snapshot 25, scene 35, renderer 67, first frame 299.

After three more opens and closes: 231 MB (after the first close: 144 MB).

Memory before opening, biggest parts: TOTAL 65 MB, Malloc Small 41 MB, Malloc Large 4 MB, Malloc Metadata 3 MB, WebKit Malloc 3 MB, Stack 2 MB.
Memory open and still, biggest parts: TOTAL 164 MB, Malloc Small 59 MB, Owned physical footprint (unmapped) (graphics) 45 MB, Malloc Large 29 MB, IOSurface 8 MB, Malloc Metadata 3 MB.
Memory animating, biggest parts: TOTAL 740 MB, Owned physical footprint (unmapped) (graphics) 589 MB, Malloc Small 61 MB, Malloc Large 45 MB, IOAccelerator (graphics) 9 MB, Owned physical footprint (unmapped) 8 MB.
Memory closed, biggest parts: TOTAL 144 MB, Malloc Small 69 MB, Malloc Large 45 MB, IOSurface 8 MB, Malloc Metadata 3 MB, WebKit Malloc 3 MB.
Memory closed after three more opens, biggest parts: TOTAL 231 MB, Malloc Small 84 MB, IOSurface 79 MB, Malloc Large 45 MB, Malloc Metadata 3 MB, WebKit Malloc 3 MB.

What the window said at the end of each case: Never opened (the app alone): no answer yet; Open, at rest (Motion Off): no answer yet; Animating, Calm: no answer yet; Animating, Motion Full (5 minutes): seen; Covered by another window: not seen; Minimised: not seen, minimised; Closed: seen.
