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
| R1 the prototype's range of models: 6 desks, 5 chairs; 8 plants in 5 pots; single, dual, wide and laptop screens with keyboards; per-desk props never the same set twice; sofas, bookcases, huddle tables, whiteboards, workbenches, lockers, beanbags, ping-pong, an arcade; kitchen, lift, directory, usage board, This Mac's rack, notice board, meeting rooms; people from parts (9 hair styles, 4 tops, glasses, beards, a badge in the App's colour) in 9 poses | `cargo test -p juggler-floor models::tests::every_model_makes_parts_on_its_floor_with_its_pick_id`; enums `DeskKind`, `ChairKind`, `Screens`, `Plant`, `Pot`, `Nook`, `Divider` in `kit.rs`; `Look::random` and `Pose` in `models.rs`; the furniture in `scene/area.rs` and `scene/building.rs` | every listed model built; a model sheet capture | the test builds all 6 desks, 5 chairs, 4 screen sets, a set of props, all 8 plants (in one pot kind only) and all 10 poses with a balloon above the head; `Look::random` draws `style` from 9 and `top_style` from 4, glasses, beards and the badge colour; the furniture words are all in the scene code. Missing: the model sheet capture the task asks for; nothing checks that no two desks get the same props (`desk_props` draws them at random); keyboards are not named in the models; only Terracotta of the 5 pots is exercised | UNPROVEN | 94a5df403 | 2026-11-06 | Claude (self) |
| R2 textures painted procedurally once into one atlas: floors, walls, screens, signs, books, posters, rugs, the skyline | `cargo test -p juggler-floor tex::tests::the_atlas_packs_once_per_key_and_says_when_full tex::tests::every_familys_floor_and_wall_paint_and_the_overlay_fits`; code: `floor_img`, `wall_img`, `screen_img`, `sign_img`, `books_img`, `poster_img`, `rug_img`, `skyline_img` in `crates/juggler-floor/src/tex.rs`; `Atlas::new(2048, 2048)` once in `Scene::build` (`scene/mod.rs`) | one atlas, every kind of texture painted from code | the atlas test asserts a key is packed once and a full atlas says so; the family test paints every family's floor and wall opaque and every screen picture; every painter listed is a function in `tex.rs` and `Scene::build` makes one atlas for the scene (the overlay's glyphs have their own, as the module says). No image is bundled | PASS | 0b2231ce7 | 2026-11-06 | Claude (self) |
| R3 a static part built once and rebuilt only on a reflow or a kit change; a moving part rebuilt each drawn frame | code: `Scene::build` fills `stat`, `Scene::frame` builds the moving part (`scene/mod.rs`, `scene/frame.rs`); `Running::message_snapshot` in `crates/juggler-app/src/features/floor/render.rs`; `cargo test -p juggler-floor scene::tests::the_moving_part_shows_each_state_and_counts_subagents` | static only on reflow or kit change | the split exists and the moving-part test passes per frame. But `message_snapshot` calls `World::build` → `Scene::build` for every published snapshot (any news: a state change, a new session), not only on a reflow or a kit change. Measured cost of that rebuild: "Building the scene ≤ 50 ms: 6.83 ms on average" (`docs/qa/floor/budgets-2026-11-06.md`). Needs either the code changed or the requirement amended | UNPROVEN | 2df52c98f | 2026-11-06 | Claude (self) |
| R4 the same snapshot and seed give the same scene, bit for bit | — | a determinism test (scene hash per seed) | no such test exists in `crates/juggler-floor` or `crates/juggler-floor-render` (searched for a scene hash and for determinism). The seeded `Rng` (`rng.rs`) is built for it, and `materials::tests::maps_are_the_same_every_time_and_sized_as_promised` covers the material maps only | UNPROVEN | 0b2231ce7 | 2026-11-06 | Claude (self) |
| Budget: the static scene for 20 Apps and 150 sessions ≤ 50 ms; the moving part ≤ 1 ms a frame | `cargo test --release -p juggler-floor scene::tests::building_twenty_apps_and_a_hundred_and_fifty_sessions_is_quick`; `docs/qa/floor/budgets-2026-11-06.md` | ≤ 50 ms build; ≤ 1 ms a frame | the test asserts 150 session picks, a build ≤ 50 ms and a frame ≤ 10 ms (looser than the 1 ms budget); the record says 6.83 ms per build on the replica (9 Apps, 340 sessions). The 1 ms moving-part budget is not measured on its own | UNPROVEN | 0b2231ce7 | 2026-11-06 | Claude (self) |
| Gate: a determinism test | — | exists, passes | missing (see R4) | UNPROVEN | — | 2026-11-06 | Claude (self) |
| Gate: `size_budget.py` (files under 1,500 lines) | `scripts/size_budget.py`, `scripts/size_budget.allow` | each file within budget or its cap | `models.rs` is over 1,500 and on the allow list at 1978; counted with the script's own `rust_lines` it is 1993 today, over its cap; the other scene files are under 1,500. The gate was not run for this manifest | UNPROVEN | 80f101da0 | 2026-11-06 | Claude (self) |
| Gate: every model has a capture in the gallery | — | one per model | the golden frames (`crates/juggler-floor-render/tests/golden/`) show the office, not each model; no model sheet | UNPROVEN | 3b983c873 | 2026-11-06 | Claude (self) |

## Binary artifacts

None yet. Binary evidence is content-addressed: `<claim>-<sha8>.png|log|json`.
Re-running a check ADDS a new file; cited artifacts are never
overwritten.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (repo paths only)
- [x] Transcript excerpts scrubbed of personal content (none)
