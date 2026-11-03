# Floor office — native wgpu prototype (round 2: low-poly 3D)

A throwaway prototype of the Floor: a small Tauri app with a minimal main window (webview) and a
native Floor window drawn with wgpu. Round 1 painted flat pixel art at one fixed angle. Round 2 is
a real 3D scene of low-poly models drawn as pixel art, so the view turns, tilts and zooms.

The scene holds twelve sample Apps with generic names on four Accounts, two floors, and a kitchen.
Docs:
- [`docs/research/floor/rules-and-mechanics.md`](../../docs/research/floor/rules-and-mechanics.md):
  which signal shows as what, layout, models and interaction.
- [`docs/research/floor/style-kits.md`](../../docs/research/floor/style-kits.md): the look of each
  family.

This is not app code. It runs on macOS only, because input and visibility come from AppKit.

## Run it

```sh
cd spikes/floor-office
nice -n 10 cargo build --release -j 4
./target/release/floor-office
```

| Do this | To |
|---|---|
| Drag, or scroll with two fingers | Pan |
| Right-drag, ⌥-drag, a two-finger twist, Q / E, arrow keys | Turn and tilt |
| Pinch, ⌘-scroll, + / − | Zoom; far out shows the building |
| H (or Floor ▸ Reset the view) | Back to the default angle |
| 0, 1, 2 (or the chips) | Building, floor 1, floor 2 |
| Hover | Outline, tooltip; people look up at you |
| Click an area, desk, person, helper, usage gauge, rack | Open it in the main window |
| Click plants, ping-pong, coffee, beanbags, arcade, the cat | Play |
| A | Red alert (demo) |
| G | One App grows: watch the areas after it move on, in order (not saved) |

The Floor menu also has **Re-create the office**: it picks new looks for every App and saves them.

The office's choices (each App's layout type and seed, and its size) are saved in
`floor-state.json` in the app's data folder, or wherever `--state PATH` says. Delete the file to
start over.

Switches: `--anim FPS` (8; 0 = nothing moves), `--alert`, `--on-top`, `--state PATH`,
`--exit-after S`, `--cover-after S`, `--minimize-after S` and `--tour DIR`. The tour captures a
scripted walk as raw RGBA frames; `measure/rgba2png.py /2 *.rgba` turns them into PNGs.

## Measure it

```sh
python3 measure/run.py > measure/out/results.json
```

One fresh launch per case: at rest, 8 fps, 30 fps, covered, minimised, then 5 minutes at 8 fps.
Each records:
- CPU % of one core for the app and its WebKit processes;
- `top` power and `footprint` memory;
- the first-frame time and the frame count.

It uses a throwaway state file. Add `--quick` to skip the 5-minute run.

## Code

| File | What it holds |
|---|---|
| `src/layout.rs` | Cells, first-fit placement in a fixed order, sizes, the saved state, desk groups packed from the aisle |
| `src/kit.rs` | The style kits: tool families, layout types, the saved choice |
| `src/scene.rs` | The office: building, areas, lobby, signals, people, props, picks |
| `src/models.rs` | Desks, chairs, plants, screens, desk props, people, the cat |
| `src/mesh.rs` | The low-poly unit shapes everything is built from |
| `src/tex.rs` | Procedural textures: floors, walls, screens, signs, books, posters; the overlay's glyphs |
| `src/render.rs` | Passes (shadow, scene and pick ids, glass, post, overlay), the orbit camera, the draw loop, the tour |
| `src/input.rs` | Mouse, trackpad and keys from AppKit |
| `src/math.rs`, `src/paint.rs` | Matrices; texture painting, colour, randomness, the pixel font |
| `src/main.rs` | The Tauri app, windows, menu and switches |
