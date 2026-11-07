# Where the Floor's memory sits while it animates (AJ-E3502 R1)

AI Juggler Dev, the release build (at c6e4a92e8's code), in a generic lab,
the Floor open on its floor and animating (Motion following the system:
people walking), the window 1781 × 1320 points on a 1× display. Read with
`footprint <pid>` and `vmmap --summary <pid>`, the system's VM accounting
(the same regions Instruments' VM Tracker lists; Instruments itself isn't
installed on this Mac, which has the Command Line Tools only).

## What the process holds

| Category (footprint) | Dirty | What it is |
|---|---|---|
| Owned physical footprint (unmapped) (graphics) | 621 MB | GPU memory the process owns: the Floor's textures and buffers, and Metal's own working memory |
| Malloc Small + Large | 86 MB | the app's heap (the webview's process is separate) |
| IOSurface | 28 MB (83 regions) | the window's drawables: the swap chain |
| IOAccelerator (graphics) | 9 MB | command and resource bookkeeping |
| Owned physical footprint (unmapped) | 10 MB | non-graphics owned memory |
| The rest | ~21 MB | stacks, `__DATA`, CoreAnimation, metadata |
| **Total** | **775 MB** (peak 792 MB) | |

## The Floor's own GPU resources, from the code

| Resource | Size | Bytes |
|---|---|---|
| Drawables (`desired_maximum_frame_latency` 2, so up to 3, BGRA8) | 3 × 1781 × 1320 × 4 | 28 MB, matching the IOSurface line |
| Shadow map (`SHADOW` 2048², Depth32) | 2048 × 2048 × 4 | 16.8 MB |
| The atlas (2048 wide, the rows in use: 1024 for a big office) | 2048 × 1024 × 4 | 8.4 MB |
| Material maps (`SIZE` 256², 16 layers, 9 mip levels) | 256² × 16 × 4 × 4/3 | 5.6 MB |
| Targets at the pixel size (colour Rgba8, ids R32, depth D32; Pixel art at 2 device pixels) | 3 × 890 × 660 × 4 | 7.0 MB (Full resolution: 28 MB) |
| Instances and the 96 lights | buffers | under 4 MB |
| **The Floor's own** | | **about 70 MB** (Full resolution: about 90 MB) |

## So the ~500 MB is

Metal's working memory for the device the Floor opens: the 621 MB of owned
graphics memory less the Floor's ~40 MB of textures and buffers inside it
(the drawables are counted as IOSurface) leaves about 580 MB, in line with
D-79's readings of a bare wgpu device on this Mac (449 MB) and a bare Metal
view (273 MB). It is held while the device lives and goes when the Floor
closes (the harness: closed, the app is back to 88–130 MB). The Floor's own
share, the budget D-79 set (≤ 250 MB animating), reads +107 to +122 MB in
the harness records.
