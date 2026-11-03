# Floor office — first native wgpu prototype

A throwaway prototype of the Floor: a small Tauri app with a minimal main
window (webview) and a native Floor window drawn with wgpu. The scene is
seven sample Apps with generic names on two floors, plus a commons. Every
area takes its look from a per-App style kit. The design is in
[`docs/research/floor/style-kits.md`](../../docs/research/floor/style-kits.md).

This is not app code. It runs on macOS only, because input and visibility
come from AppKit.

## Run it

```sh
cd spikes/floor-office
nice -n 10 cargo build --release -j 4
./target/release/floor-office
```

| Do this | To |
|---|---|
| drag, or scroll with two fingers | pan |
| pinch, ⌘-scroll, or `+` / `-` | zoom: building, floor, area, desk |
| `0`, `1`, `2` | see the building, floor 1 or floor 2 |
| click an area or a person | open that App or session in the main window |
| `R` | toggle the red alert (demo) |

The Floor menu has the same views and a way back to the main window.

Switches: `--anim FPS` (8; 0 = nothing moves), `--alert`, `--on-top`,
`--exit-after S`, `--cover-after S`, `--minimize-after S` and
`--tour DIR`. The tour captures a scripted walk through the zoom levels as
raw RGBA frames; `measure/rgba2png.py` turns them into PNGs.

## Measure it

```sh
python3 measure/run.py > measure/out/results.json
```

The script runs one fresh launch per case: at rest, 8 fps, 30 fps, covered,
minimised, then 5 minutes at 8 fps. It records CPU % of one core for the
app and its WebKit processes, `top` power, `footprint` memory, the first
frame time and the frame count. Add `--quick` to skip the 5-minute run.

## Code

| File | What it holds |
|---|---|
| `src/kit.rs` | The style kits: tool families, seeded variants, mark colours |
| `src/world.rs` | The procedural building: per-pixel floors and walls, signs, areas, people, states, hit areas, the sprite atlas |
| `src/people.rs` | People from parts, and their poses |
| `src/paint.rs` | The software painter, camera, colour ramps, noise and the 5×7 font |
| `src/render.rs` | The wgpu renderer, the zoom levels, the on-demand draw loop and the tour |
| `src/input.rs` | Mouse, trackpad and keys from AppKit |
| `src/main.rs` | The Tauri app, windows, menu and switches |
