# Evidence manifest — AJ-E3507 The renderer: the pixel look, picking and the orbit camera

The renderer is `crates/juggler-floor-render` (`gpu.rs`: the four passes;
`shaders.rs`: WGSL; `lib.rs`: `Renderer`, `Look`, picking, offscreen
capture). The camera is pure and tested in `crates/juggler-floor/src/camera.rs`,
its keys and tweens in `driver.rs`. Golden frames are rendered offscreen
by `crates/juggler-floor-render/tests/golden.rs` into
`crates/juggler-floor-render/tests/golden/` (building, floors, `turn-0…3`,
`tilt-low`, `tilt-high`, areas, hover, red alert, and the `full-*` set).

Two later rulings change details of this task's text: D-86 (the default
look, a finer pixel on Retina) and AJ-E3516 R5 (zoom never switches
between a floor and the building; the lift does). The task text has not
been amended for either.

Not here yet: the task's evidence proof, a recording of a full turn, tilt
and zoom on the release build (the golden frames are offscreen stills).

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 the passes: the sun's shadow; the scene at the pixel size writing colour, depth and a pick id; glass and the floor logos, blended; post (outlines, the hovered thing outlined, sky, alert wash, vignette); a crisp overlay with its own glyph atlas | code: module doc and `Pipelines` in `crates/juggler-floor-render/src/gpu.rs` (shadow, scene with an `R32Uint` id target, glass with alpha blending and the id target's write mask empty, post, overlay), post shader in `shaders.rs`; `scene_px`/`pixel_size`; the logo is a `glass: true` item in `scene/area.rs::area_floor`; `tex::ui_atlas`; `cargo test -p juggler-floor-render --test offscreen it_draws_the_office_offscreen_and_picks_a_desk`; `cargo test -p juggler-floor-render --test golden the_gallery_of_golden_frames` | every pass present | each pass is in the code and the golden frames show outlines, sky, the red-alert wash (`red-alert.png`), the hover outline (`hover.png`) and the overlay's chips. Pixel size: 1/2 on 1×; on Retina 1/3 in 0.4.0's look and 1/2 in the default look, as D-86 ruled ("two device pixels, not three, never under two"). The offscreen test asserts a real picture (> 40 colours) | PASS | 94a5df403 | 2026-11-06 | Claude (self) |
| R2 the sun in three bands; up to 96 point lights (the nearest) add lamps, pendants, fires and beacons | code: `MAX_LIGHTS = 96` and the nearest-first sort in `Gpu::write_frame` (`gpu.rs`); the banding in `shaders.rs` (`floor(ndl * sh * 3.0 + 0.4) / 3.0`) | three bands; ≤ 96 nearest lights | 96 nearest lights: as specified. Bands: three only in 0.4.0's look; the default look since D-86 uses four (`g.look.x > 0.5`), softened further with materials on (`g.mat`). D-86 adopts the reworked light but does not name a band count, so the text and the default differ | UNPROVEN | 94a5df403 | 2026-11-06 | Claude (self) |
| R3 picking reads the id under the pointer from the last frame, with no new frame | code: `Gpu::pick` copies one texel of the last frame's id texture to a buffer (no draw) in `gpu.rs`; `cargo test -p juggler-floor-render --test offscreen it_draws_the_office_offscreen_and_picks_a_desk`; the golden test's pick per mode | a known id under a known pixel | the offscreen test projects a seated session's anchor to the screen and asserts a pick within ±12 px returns `Target::Session("s:1")`, and that a pick off-screen is 0; the golden test asserts both looks pick the same non-zero id under the hover pointer, switched four times. Both tests skip without a GPU on CI, and run on a Mac | PASS | 0b2231ce7 | 2026-11-06 | Claude (self) |
| R4 the camera per SPEC §9.7: pan, turn (free), tilt 18°–79°, zoom towards the pointer, the building when far out, the floor under the pointer coming back in, H to reset; tweens ease over 0.45 s | `cargo test -p juggler-floor camera::tests::turning_keeps_the_tilt_within_its_limits_and_tweens_go_the_short_way camera::tests::zooming_never_switches_between_a_floor_and_the_building camera::tests::home_frames_a_floor_and_the_building_from_above driver::tests::a_camera_move_draws_until_it_lands`; `PITCH_MIN = 0.32`, `PITCH_MAX = 1.38`, `TWEEN_S = 0.45` in `camera.rs`; `Key::Char('h')` in `driver.rs` | each motion and limit | tilt clamps at 0.32 rad (18.3°) and 1.38 rad (79.1°); pan moves the target; a turn tween goes the short way; a 0.45 s move draws 25–35 frames. Zoom moves toward the ground under the pointer (`View::zoom_at`, code only). The "building when far out, floor coming back in" part is replaced by AJ-E3516 R5: the test asserts zoom never leaves a floor or enters one. H calls `Frame::home` (code only, no test). Needs the task text amended to AJ-E3516's camera | UNPROVEN | 94a5df403 | 2026-11-06 | Claude (self) |
| R5 walls, and what hangs on them, vanish when the camera is behind them, and cast no shadow then | code: `culled()` in `shaders.rs`, applied in `vs` and `vs_shadow`, for items flagged `models::CUTAWAY` (walls in `scene/building.rs`, posters, boards and wall faces in `scene/area.rs`); golden `turn-0.png`…`turn-3.png` | gone from view and from the shadow map | an item with the cutaway flag whose face points away from the eye is moved out of clip space in both the scene and the shadow vertex shaders; in the four turn frames the near walls are open from every side (checked by eye on `turn-2.png`). No unit test of the cull itself | PASS | 94a5df403 | 2026-11-06 | Claude (self) |
| Gate: golden frames rendered offscreen | `cargo test -p juggler-floor-render --test golden the_gallery_of_golden_frames` | frames committed, compared | 28 PNGs in `tests/golden/`, including the turn sequence and both tilt extremes; not re-run for this manifest | UNPROVEN | 3b983c873 | 2026-11-06 | Claude (self) |
| Gate: a picking test | offscreen test above | passes | exists (see R3); not re-run for this manifest | UNPROVEN | 0b2231ce7 | 2026-11-06 | Claude (self) |
| Gate: a camera-limits test | camera tests above | passes | exists (see R4); not re-run for this manifest | UNPROVEN | 94a5df403 | 2026-11-06 | Claude (self) |
| Budget: ≤ 1.5 ms of CPU a frame on the render thread | `docs/qa/floor/budgets-2026-11-06.md` (release build, replica: 9 Apps, 340 sessions) | ≤ 1.5 ms | "0.60 ms (2406 frames)" animating in Pixel art; 0.71 ms in Full resolution (`budgets-2026-11-06-full.md`) | PASS | fd2637851 | 2026-11-06 | Claude (self) |
| Budget: GPU ≤ 8 ms a frame on the lowest supported Mac | — | ≤ 8 ms | not measured: the records are from one Apple M5 Ultra; no GPU frame time on the lowest supported Mac | UNPROVEN | — | 2026-11-06 | Claude (self) |
| Evidence: a recording of a full turn, tilt and zoom on the release build | — | recorded | not recorded | UNPROVEN | — | 2026-11-06 | Claude (self) |

## Binary artifacts

None added here. The golden frames cited live in
`crates/juggler-floor-render/tests/golden/`. Binary evidence is
content-addressed: `<claim>-<sha8>.png|log|json`. Re-running a check ADDS
a new file; cited artifacts are never overwritten.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (repo paths only)
- [x] Transcript excerpts scrubbed of personal content (none; the golden company uses generic names)
