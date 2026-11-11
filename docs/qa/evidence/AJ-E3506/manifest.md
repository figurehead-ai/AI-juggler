# Evidence manifest — AJ-E3506 Models, textures and the scene

The task file still says TODO, but the work is in the tree:
`crates/juggler-floor/src/{mesh,models,tex,paint,materials}.rs` and
`crates/juggler-floor/src/scene/` (`mod.rs`, `area.rs`, `building.rs`,
`frame.rs`, `tower.rs`, tests in `scene/tests.rs`). The kinds of desk,
chair, screen, plant and pot are enums in `kit.rs`.

What is missing for this task: the model sheet capture (the task's
evidence proof and its visual testing; no sheet exists in
`crates/juggler-floor-render/tests/golden/` or `docs/qa/evidence/_gallery/`),
and the determinism test (scene hash per seed). `docs/qa/0.4-completion.md`
lists both as still to do.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 the prototype's range of models: 6 desks, 5 chairs; 8 plants in 5 pots; single, dual, wide and laptop screens with keyboards; per-desk props never the same set twice; sofas, bookcases, huddle tables, whiteboards, workbenches, lockers, beanbags, ping-pong, an arcade; kitchen, lift, directory, usage board, This Mac's rack, notice board, meeting rooms; people from parts (9 hair styles, 4 tops, glasses, beards, a badge in the App's colour) in 9 poses | `cargo test -p juggler-floor models::tests::every_model_makes_parts_on_its_floor_with_its_pick_id scene::tests::every_pot_holds_a_plant_and_no_two_desks_share_their_props`; the model sheet `AJ_FLOOR_EVIDENCE=… cargo test -p juggler-floor-render --test golden the_model_sheet -- --ignored` → `models-{desks,plants,people}-{pixel,full}-<sha8>.png` here; the furniture in the area frames (`tests/golden/area-00…10.png`) and the lobby in `building.png`, `floor-1.png` | every listed model built and shown | tests pass: all 6 desks, 5 chairs, 4 screen sets, every plant in every pot, all 10 poses. Sheet: six desks each with its chair, screens and keyboard and its props; eight plants in the five pots, a fern in each pot; ten people (each pose; nine hair styles, four tops, glasses, beards, the badge) facing the camera. **Found and fixed here:** props were drawn at random per desk, and 24 desks of an area drew only 23 sets; a repeated set is drawn again from the desk's seed (1a0cca434), and the test asserts 24 of 24 | PASS | 3c8d6a9b2 | 2026-11-06 | Claude (self) |
| R2 textures painted procedurally once into one atlas: floors, walls, screens, signs, books, posters, rugs, the skyline | `cargo test -p juggler-floor tex::tests::the_atlas_packs_once_per_key_and_says_when_full tex::tests::every_familys_floor_and_wall_paint_and_the_overlay_fits`; code: `floor_img`, `wall_img`, `screen_img`, `sign_img`, `books_img`, `poster_img`, `rug_img`, `skyline_img` in `crates/juggler-floor/src/tex.rs`; `Atlas::new(2048, 2048)` once in `Scene::build` (`scene/mod.rs`) | one atlas, every kind of texture painted from code | the atlas test asserts a key is packed once and a full atlas says so; the family test paints every family's floor and wall opaque and every screen picture; every painter listed is a function in `tex.rs` and `Scene::build` makes one atlas for the scene (the overlay's glyphs have their own, as the module says). No image is bundled | PASS | 0b2231ce7 | 2026-11-06 | Claude (self) |
| R3 (amended) a static part rebuilt only when what it shows changes (a published snapshot, news only), a moving part each drawn frame | code: `Scene::build` fills `stat`, `Scene::frame` builds the moving part (`scene/mod.rs`, `scene/frame.rs`); `Running::message_snapshot` (`render.rs`) builds on a published snapshot only; the feed publishes only news (`differs`, at most 4 a second); `cargo test -p juggler-app --lib features::floor::tests::a_reading_that_wiggles_between_drawn_steps_is_not_news`; budgets `docs/qa/floor/budgets-2026-11-06.md` | the static part only on news; within the 50 ms build budget | as amended: rebuilt only for news (a reading that wiggles between drawn steps isn't), 6.83 ms on average, under the 50 ms budget; the moving part per frame (`the_moving_part_shows_each_state_and_counts_subagents`). The text amended: the static part shows each session's desk, sign and board, so a state change does rebuild it | PASS | 1a0cca434 | 2026-11-06 | Claude (self) |
| R4 the same snapshot and seed give the same scene, bit for bit | `cargo test -p juggler-floor --lib scene::tests::the_same_snapshot_and_seed_give_the_same_scene` | two builds with one seed equal in every field (meshes, lights, picks, labels, atlas pixels; floats by their exact shortest form; the overlay's hash-map tables compared sorted); another seed differs | passes: seeds 7 and 11 each give the same scene twice, and 7 and 11 differ | PASS | ca8a3de8c | 2026-11-06 | Claude (self) |
| Budget: the static scene for 20 Apps and 150 sessions ≤ 50 ms; the moving part ≤ 1 ms a frame | `cargo test --release -p juggler-floor scene::tests::building_twenty_apps_and_a_hundred_and_fifty_sessions_is_quick`; `docs/qa/floor/budgets-2026-11-06.md` | ≤ 50 ms build; ≤ 1 ms a frame | the test asserts 150 session picks, a build ≤ 50 ms and a frame ≤ 10 ms (looser than the 1 ms budget); the record says 6.83 ms per build on the replica (9 Apps, 340 sessions). The 1 ms moving-part budget is not measured on its own | UNPROVEN | 0b2231ce7 | 2026-11-06 | Claude (self) |
| Gate: a determinism test | `scene::tests::the_same_snapshot_and_seed_give_the_same_scene` | exists, passes | passes (see R4) | PASS | ca8a3de8c | 2026-11-06 | Claude (self) |
| Gate: `size_budget.py` (files under 1,500 lines) | `python3 scripts/size_budget.py` | each file within budget or its cap | `size-budget: OK (14 file(s) over budget, capped)`: `models.rs` back under its cap since the glow code moved to `glow.rs` (2f2c77bd4) | PASS | 1a0cca434 | 2026-11-06 | Claude (self) |
| Gate: every model has a capture in the gallery | `the_model_sheet` (ignored, writes on request) → `models-*.png` here; the golden frames for the areas' furniture and the lobby | one per model | every desk, chair, screen set, plant, pot, pose and look on the sheet; the furniture of each family in `area-00…10.png`; the lobby (kitchen, lift, directory, board, rack, notice board) in `floor-1.png` and `building.png` | PASS | 3c8d6a9b2 | 2026-11-06 | Claude (self) |

## Binary artifacts

The model sheet (1600×1000 at 2×, the generic company's office emptied,
the models on a plain floor beside it), both looks:

- `models-desks-full-e3c7af37.png`
- `models-desks-pixel-e53c3c7c.png`
- `models-people-full-9269c038.png`
- `models-people-pixel-fd7b2ff6.png`
- `models-plants-full-8e8c5b6e.png`
- `models-plants-pixel-85849441.png`

Binary evidence is content-addressed: `<claim>-<sha8>.png|log|json`.
Re-running a check ADDS a new file; cited artifacts are never
overwritten.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (repo paths only)
- [x] Transcript excerpts scrubbed of personal content (none)
