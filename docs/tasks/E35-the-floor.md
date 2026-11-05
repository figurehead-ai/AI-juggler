# E35 — The Floor: the whole company as a live office, in its own window (0.4, D-67, D-68)

> Source: the maintainer, 2026-11-03, approving the second prototype,
> verbatim: "You know what it would be great if on the floor of the area,
> as part of the carpet or floor you overlay 30 or 40% transparency logo of
> the app so it is easily recognizable . But otherwise looks great, let's
> go with it. Make it actually work within 0.5.0 , plan and spec it out
> fully." Then: "I mean logo like : Codex, Claude Code along with initials
> of the account". The earlier rulings of the same day (native wgpu; each
> area distinct and always its tool's look; a sign per area; procedural;
> more model detail; packed areas, 10 Apps a floor as a setting, at most
> two a column; stable order; a look that sticks; fun touches; "the main
> purpose of everything is to be an indicator") are quoted in D-67.
>
> **What this epic builds.** SPEC §9.7, the way ADR-0026 says. The look and
> the rules come from the approved prototype: `docs/research/floor/`
> (`rules-and-mechanics.md`, `style-kits.md`) and the spike on the branch
> `proto/floor-office-2` (`spikes/floor-office/`), which is the reference
> for every model, layout and budget below. The spike is research code:
> it is rewritten to the workspace's standards, not copied.
>
> **Order.**
> - AJ-E3501 and AJ-E3503 first, beside each other (the window and the
>   model need nothing from each other). AJ-E3502 straight after AJ-E3501.
> - AJ-E3504 and AJ-E3505 on the model.
> - AJ-E3506, then AJ-E3507.
> - AJ-E3508, AJ-E3509 and AJ-E3511 beside each other.
> - AJ-E3510, then AJ-E3512.
> - AJ-E3513 gates all of it.
> - AJ-E3514 is last: the Floor is done when the walks pass, in 0.4 (D-68: moved from 0.5.0).
>
> **Rules that hold for every task.**
> - Original art only, generated from code. No bundled brand image: tool
>   marks are drawn as the main window draws them, icons are read at run
>   time (D-38).
> - Generic names in every capture and fixture.
> - Colour is never the only signal.
> - Nothing is drawn while nothing changes, or while the window can't be
>   seen.
> - No branch on a tool outside its adapter (ADR-0019, `just
>   tool-literals`).
> - No new macOS-only code outside a seam (`just platform`).
> - Unsafe code only in `juggler-window-events`.
> - Each task names its scenarios in [`docs/qa/0.4-validation-plan.md`](../qa/0.4-validation-plan.md)'s
>   form and isn't done until they pass by hand on the release build.

### AJ-E3501 The Floor window: crates, a native surface, input and the draw-on-change loop | Status: IN-PROGRESS | Priority P0 | Effort L | Depends: —
**Goal**: A second, native window opens from the main window, draws with wgpu, takes mouse, trackpad and keys, and draws only when something changes and it can be seen.
**User story**: As someone who wants the Floor open all day, I open it with ⌘⇧M, and it costs nothing when nothing moves or when it's behind other windows.
**Requirements**:
1. Three crates SHALL be added as ADR-0026 says:
   - `juggler-floor` (pure, no GPU);
   - `juggler-floor-render` (wgpu, safe surface creation from the Tauri window);
   - `juggler-window-events` (the audited carve-out, macOS implementation, `Unsupported` elsewhere).

   Each comes with its `layering.toml` edges, `cargo deny` green, and a PR note with the alternative analysis (QUALITY §3).
2. The window SHALL come from Tauri's `unstable` window builder (Tauri pinned exactly), labelled `floor`, created on first open and destroyed on close, which drops the device and surface.
3. A render thread SHALL wake for:
   - input;
   - a camera tween;
   - a snapshot change;
   - an animation tick, only while something moving is in view and the window is visible.

   Visibility is `occlusionState` and minimised, polled at 2 Hz only while the window draws or is hidden. A refused frame SHALL be retried every 100 ms while visible, never dropped.
4. Entry points SHALL be:
   - a Floor button in the header beside the user menu;
   - Window › The Floor in the menu bar;
   - the tray menu;
   - ⌘⇧M, which also brings the main window back from the Floor.

   On Windows and Linux the entry SHALL say it isn't available on that system yet, and why.
5. Settings › The Floor › "Open the Floor when AI Juggler starts" SHALL open it at start; otherwise nothing of the Floor runs until first opened (lazy).
**UI/UX reasoning**: desktop first (a second window you can keep on another display); lazy-load everywhere; always on without cost.
**Performance budget**: SPEC §9.7: 0 frames at rest after the first, 0 within 1 s of being covered or minimised, first frame ≤ 400 ms.
**Quality gates**:
- A headless clock test: a scripted sequence of messages gives exactly the expected frames (none at rest, none hidden, one per change).
- `just structure`, `just platform` and `just verify-static` green, the lint carve-out recorded.
- The three crates build and lint in `build-linux` and `build-windows`.
**Evidence/validation proof**: the frame log of a 60 s run at rest, covered and minimised (`docs/qa/evidence/AJ-E3501/`).
**Visual testing**: n/a (an empty floor is enough here).
**Files to touch**: `crates/juggler-floor/`, `crates/juggler-floor-render/`, `crates/juggler-window-events/` (new); `Cargo.toml` (members, lint carve-out comment); `docs/plan/layering.toml`; `crates/juggler-app/src/features/floor/` (new: window, commands); `crates/juggler-app/src/quit.rs` (menu), the tray; `ui/src/components/AppHeader.tsx`, `ui/src/lib/shortcuts.ts`, `ui/src/lib/app-keys.ts`.
**Research basis**: ADR-0026; the spike's `render.rs` (loop, retry), `input.rs`, `main.rs`; `floor-spike-results.html`.
**Handover note**: The three crates are in: `juggler-floor` (pure), `juggler-floor-render` (wgpu, safe `create_surface`), `juggler-window-events` (the audited carve-out: four unsafe calls, macOS; Unsupported with why elsewhere). Tauri pinned `=2.12.4` with `unstable`. `features/floor/window.rs` makes the window on first open (surface and monitor on the main thread) and drops it on close; `render.rs` runs the driver (`juggler-floor/src/driver.rs`, headless clock tests: 1 frame in 60 s at rest, none hidden, one per change, a refused frame retried every 100 ms). Entry points: the header's Floor button, Window › The Floor and the tray (⌘⇧M, which brings the main window back from the Floor); open-at-start reads `floor.open_at_start`. Left: the frame-log evidence of a 60 s run at rest, covered and minimised (`docs/qa/evidence/AJ-E3501/`), and the Linux/Windows build check in CI.

### AJ-E3502 Memory settled: the drawables held while animating | Status: IN-PROGRESS | Priority P0 | Effort S | Depends: AJ-E3501
**Goal**: The Floor meets its memory budget while animating, not only at rest.
**User story**: As someone on a 16 GB Mac, opening the Floor doesn't cost half a gigabyte.
**Requirements**:
1. The ~500 MB held while frames are presented (round 2's measurement) SHALL be explained with Instruments' allocations: drawables, IOSurfaces, the shadow map, the targets.
2. The fix SHALL bring "memory added while animating" within SPEC §9.7's ≤ 250 MB. The candidates:
   - `desired_maximum_frame_latency` 1;
   - drawable size equal to the window;
   - the low-resolution targets sized to the pixel size;
   - a smaller shadow map;
   - releasing the surface after 30 s without a frame.
3. Until it does, the surface SHALL be released after 30 s without a frame, and the gap SHALL be named in the 0.4.0 release notes' Known issues.
**UI/UX reasoning**: always on means always affordable.
**Performance budget**: SPEC §9.7 memory rows.
**Quality gates**: the harness's memory reading at 30, 150 and 290 s stays flat and within budget.
**Evidence/validation proof**: the Instruments trace summary and the before/after readings (`docs/qa/evidence/AJ-E3502/`).
**Visual testing**: n/a.
**Files to touch**: `crates/juggler-floor-render/`.
**Research basis**: round 2's measurements in `docs/research/floor/README.md`; the spike's `measure/run.py`.
**Handover note**: measured, not yet within budget (`docs/qa/floor/budgets-2026-11-04.md`, release build, M5 Ultra 96 GB). The ~550 MB held while frames come is Metal's own per-process working memory ("Owned physical footprint (unmapped) (graphics)"): a bare Metal clear of a 64×64 texture, with no wgpu, holds 273 MB on this Mac; wgpu's bare device 449 MB; the Floor 546 MB. The command queue's size makes no difference (8, 64, 4096 tried). Metal frees it within a second of the last frame, which is why at rest, covered and minimised read 170–246 MB in all. Everything the Floor itself creates on the GPU is about 40 MB (atlas 16, shadow map 16, instances 7.7). So none of the candidates in req. 2 can reach ≤ 250 MB while animating on this GPU; the budget needs a ruling (the Floor's own memory, with Metal's working pool named per Mac). After closing, +78 MB stays (Metal's own residue is ~7 MB): the open-and-close cycles in the harness tell a leak from the allocator's cache.

### AJ-E3503 The office model: cells, order, sizes, floors and saved choices | Status: IN-PROGRESS | Priority P0 | Effort L | Depends: —
**Goal**: One pure model places every App on stable cells and every session at a stable desk, and keeps each App's look once chosen.
**User story**: As someone with twenty Apps, my areas stay where I learnt them; when one grows, the others move on in order, never jumbled.
**Requirements**:
1. Floors, cells, sizes (S, M, L, annexes beyond 24), first-fit order, the floor limit (4–16, default 10) and free cells SHALL follow SPEC §9.7 exactly.
2. Order SHALL be the order the Floor first saw each App. Growth SHALL apply at once. Shrinking SHALL wait until a day below the smaller size. A closed App's cells SHALL stay empty until the next reflow or Re-create.
3. Per App, order, layout type, seed, size and `fits_smaller_since` SHALL be saved in a new `floor_areas` table (a schema migration) the first time the Floor sees the App. The office seed SHALL be saved in `app_settings` (`floor.seed`).
4. Desk plans SHALL place groups by layout type from the aisle inwards, deterministically from the seed. A crowded area SHALL give up its nook, then use pods, so every session has a desk. Desk *i* SHALL belong to session *i* while the area keeps its size.
5. Re-create SHALL replace the office seed and every App's choice and re-place every App in the same order. It SHALL change nothing outside the Floor.
**UI/UX reasoning**: stability over variety; recognisable places (wayfinding research in `style-kits.md`).
**Performance budget**: placing 20 Apps and 150 sessions ≤ 5 ms; a reflow ≤ 2 ms.
**Quality gates**:
- Unit tests for every size, annex, floor overflow and free cell.
- A property test: random sequences of new Apps, growth and endings never move an area before the changed App, and never move a seated session unless its area changed size.
- A test that every session always has a desk.
- The migration test.
**Evidence/validation proof**: the tests; a table of plans for the prototype's sample that matches the prototype.
**Visual testing**: n/a.
**Files to touch**: `crates/juggler-floor/src/layout/` (new); `crates/juggler-core/src/storage/` (the `floor_areas` migration and repo).
**Research basis**: the spike's `layout.rs` (`pack`, `plan_area`, `pack_groups`, `group_shape`); `rules-and-mechanics.md` §3.
**Handover note**: built in `crates/juggler-floor` (`layout.rs`, tests in `layout_tests.rs`): sizes S/M/L and annexes, first-fit order, the 4–16 floor limit, free cells (the first a kitchen), grow at once and shrink after a day, closed Apps' cells kept until the next reflow, Re-create, and desk plans that give up the nook, then use pods. An annex never goes on a floor before its App's, and shared cell edges are computed from whole cells (a property test found both). Storage: `floor_areas` (schema v46) and `FloorRepo` in `juggler-core` (`storage/floor.rs`, tests `tests/floor_storage.rs`); the seed key is `floor.seed`. Budget on the release build: placing 20 Apps and 150 sessions 0.27 ms, a reflow 5 µs. Left: saving each App the first time the Floor sees it, which comes with the feed (AJ-E3505), and the prototype-sample table for the evidence.

### AJ-E3504 Style families as tool data, and each area's kit | Status: IN-PROGRESS | Priority P0 | Effort M | Depends: AJ-E3503
**Goal**: Each tool's areas share a family look, each App's area is its own variant, and no code branches on a tool.
**User story**: As someone with two Claude Code Apps, both areas read as Claude Code's workshop, and I can still tell them apart.
**Requirements**:
1. `ToolSpec` (ADR-0019) SHALL name a style family:
   - Claude Code: workshop;
   - the Claude app: studio;
   - Codex: industrial;
   - OpenCode: maker garage;
   - Gemini CLI: campus;
   - Cursor: lab;
   - a neutral office for tools that name none.

   The adapter conformance kit (AJ-E1521) SHALL check that every tool names a family or knowingly takes the neutral one.
2. The family catalog in `juggler-floor` SHALL hold each family's floors, walls, desk models, chairs, plants and pots, dividers, nooks, signs, light warmth and layout types.
3. A kit SHALL be built from the family and the App's saved choice. Big things vary between kits, props per desk. The App's colour (its colour slot) marks its sign, flag, rug, badges and logo ring.
4. The Rust copy of `MARK_COLOURS` SHALL be tested against `ui/src/lib/mark-hues.ts`.
**UI/UX reasoning**: vary big things, not small ones (the oatmeal rule); same tool, same family.
**Performance budget**: building every kit ≤ 1 ms.
**Quality gates**:
- `just tool-literals` green.
- A test that two Apps of one tool differ in at least two of floor, wall, layout and sign, and two tools differ in family.
- The colour-table test.
**Evidence/validation proof**: a kit table for the sample, matching the prototype's log.
**Visual testing**: covered by AJ-E3513's gallery.
**Files to touch**: `crates/juggler-adapters/src/` (each `ToolSpec`), `crates/juggler-adapter/` (the field), `crates/juggler-floor/src/kit/` (new).
**Research basis**: `style-kits.md`; the spike's `kit.rs`.
**Handover note**: `ToolSpec.floor` (`FloorFamilies`: a word for the tool's CLI Apps and, where it differs, its desktop app's) in `juggler-adapter`; Claude Code workshop with the Claude app studio, Codex industrial, OpenCode maker garage. Grok Build and Kimi Code take `neutral` on purpose, keeping campus and lab for Gemini CLI and Cursor when their adapters come. The conformance kit checks every adapter names a word (`names_a_floor_family`); `juggler-app/tests/floor_families.rs` checks the Floor knows each word. The catalog and kits are in `juggler-floor/src/kit.rs` (one module, not a folder: it fits the budget); the colour table is tested against `mark-hues.ts`. Left: the colour slot on sign, flag, rug, badges and logo ring belongs to the scene (AJ-E3506), and the kit table for the evidence.

### AJ-E3505 The feed: one snapshot from the backend, for the Floor and its list | Status: IN-PROGRESS | Priority P0 | Effort L | Depends: AJ-E3503
**Goal**: The Floor and the main window's Floor list read one snapshot of the company, kept current by the events the app already emits.
**User story**: As someone glancing at the Floor, what it shows is what the rail says, within a breath of it changing.
**Requirements**:
1. `features/floor` SHALL build a `FloorSnapshot` from `AppState`:
   - Apps with colour slots, tools and Accounts;
   - sessions with status, needs (question or permission), subagents running, tasks, project;
   - other apps' conversations;
   - pools and their members;
   - each Account's usage left and reset;
   - the process table;
   - This Mac's health and alerts.
2. It SHALL rebuild on the existing events (`session://updated`, `session://quota`, `processes://changed`, `machine://this-mac`, `tasks://changed`, `notifications://changed`, `conversations://changed`, the Apps' change), coalesced to at most 4 a second. It SHALL send the snapshot to the render thread, and to the main window as `floor://changed`, with a `floor_snapshot` command for the list.
3. New inputs SHALL be added, each with a test:
   - an App's last active time (from `last_seen_ms`);
   - an Account's last use;
   - a per-App CPU share (from the process table);
   - how long This Mac's overload alert has been on (from `since_ms`).
4. The mapping from session data to a state SHALL match the rail. One fixture of (input, state, word) cases SHALL be read by both the Rust tests and `rail-items.ts`'s tests.
**UI/UX reasoning**: honest states (only what the backend knows); one source of truth for the canvas and the list.
**Performance budget**: building a snapshot of 20 Apps and 150 sessions ≤ 2 ms; a change reaches the screen ≤ 100 ms.
**Quality gates**:
- The shared fixture passes in both languages.
- `just structure` (IPC audit) green for the new command and event.
- No polling added (`polling_audit.py`).
**Evidence/validation proof**: a trace of a session going from working to waiting to working, on the Floor and the rail side by side.
**Visual testing**: n/a.
**Files to touch**: `crates/juggler-app/src/features/floor/feed.rs` (new), `crates/juggler-floor/src/snapshot.rs` (new), `ui/src/lib/rail-items.ts` (its test reads the fixture), `ui/src/lib/ipc/floor.ts` (new).
**Research basis**: ADR-0026 §5–6; the survey of the app's data in this epic's review notes.
**Handover note**: `FloorSnapshot` and its parts live in `juggler-floor/src/snapshot.rs` (one type for the renderer and the list; TS types generated from it). `juggler-app/src/features/floor`: `feed::gather` reads the sources the rail and pages read (sessions from the last sample, the conversation index, stored Apps and Accounts, each folder's usage, pools' placements, the process graph, This Mac's cached health, the notification summary) and `feed::assemble` builds the snapshot purely (tests in `floor/tests.rs`, with the ≤ 2 ms budget). Lazy: the handle is kept at setup; the first `floor_snapshot` starts listening to the existing events, and a 250 ms pass rebuilds, then publishes to the render thread's channel and emits `floor://changed` only when something changed. New inputs: an App's last activity, an Account's last use (its sessions' latest activity), a per-App CPU share with since-when it's been hot (half the Mac for 2 minutes), the alert's `since_ms`. The state mapping is `juggler-floor/src/state.rs`, held to the rail by `ui/src/lib/fixtures/session-states.json` (Rust and `session-states.test.ts`). The rail's snooze is the window's own, so the Floor shows a snoozed ask as asking. Also served the E30 pool reads over the dev bridge. Left: the trace evidence (working → waiting → working, Floor and rail side by side), with AJ-E3507.

### AJ-E3506 Models, textures and the scene | Status: TODO | Priority P0 | Effort L | Depends: AJ-E3503, AJ-E3504
**Goal**: The office is built from shaped low-poly models: desks look like desks, plants like plants, people like people.
**User story**: As someone zooming in, I see task chairs with casters, a monstera's split leaves, a lamp lighting a desk, and a person typing.
**Requirements**:
1. The mesh library and models SHALL match the prototype's range:
   - 6 desk models and 5 chair types;
   - 8 plants in 5 pots;
   - single, dual, wide and laptop screens with keyboards;
   - per-desk props (never the same set twice);
   - sofas, bookcases, huddle tables, whiteboards, workbenches, lockers, beanbags, ping-pong, an arcade cabinet;
   - a kitchen, the lift, directory, usage board, This Mac's rack, notice board and meeting rooms;
   - people built from parts with 9 hair styles, 4 top styles, glasses and beards, wearing a badge in the App's colour, with poses: typing, idle, waving, scratching, hands on head, stretching, asleep, helping, walking.
2. Textures SHALL be painted procedurally once into one atlas: floors, walls, screens, signs, books, posters, rugs, the skyline.
3. The scene SHALL be split into a static part, built once and rebuilt only on a reflow or a kit change, and a moving part rebuilt each drawn frame.
4. The same snapshot and seed SHALL give the same scene, bit for bit.
**UI/UX reasoning**: "plant should look like a plan, desk like a desk" (D-67); characters at the highest contrast, background calmer.
**Performance budget**: building the static scene for 20 Apps and 150 sessions ≤ 50 ms; the moving part ≤ 1 ms a frame.
**Quality gates**:
- A determinism test (scene hash per seed).
- `size_budget.py` (files under 1,500 lines).
- Every model has a capture in the gallery.
**Evidence/validation proof**: a model sheet capture (every desk, chair, plant and pose).
**Visual testing**: the model sheet in AJ-E3513's gallery, critiqued against the design-quality review.
**Files to touch**: `crates/juggler-floor/src/{mesh,models,scene,tex}/` (new).
**Research basis**: the spike's `mesh.rs`, `models.rs`, `scene.rs`, `tex.rs`.
**Handover note**: —

### AJ-E3507 The renderer: the pixel look, picking and the orbit camera | Status: IN-PROGRESS | Priority P0 | Effort L | Depends: AJ-E3501, AJ-E3506
**Goal**: The scene is drawn as crisp pixel art with light and shadow, every pixel knows what it shows, and the view turns, tilts and zooms within limits.
**User story**: As someone looking for a team, I turn the floor round to see behind a wall, zoom to a desk, and press H to come back.
**Requirements**:
1. The passes SHALL be:
   - the sun's shadow;
   - the scene at the pixel size (1/3 resolution on Retina, 1/2 on 1×), writing colour, depth and a pick id;
   - glass and the floor logos, blended;
   - post (outlines where one object stands in front of another; the hovered thing outlined in light; sky; the alert wash; a soft vignette);
   - a crisp screen-space overlay (chips, balloons, tooltips, the floor switcher) with its own glyph atlas.
2. The sun SHALL shade in three bands, and up to 96 point lights (the nearest) SHALL add lamps, pendants, fires and beacons.
3. Picking SHALL read the id under the pointer from the last frame, with no new frame.
4. The camera SHALL follow SPEC §9.7: pan, turn (free), tilt (18°–79°), zoom towards the pointer, the building when far out, the floor under the pointer coming back in, H to reset. Tweens ease over 0.45 s.
5. Walls, and what hangs on them, SHALL vanish when the camera is behind them (and cast no shadow then).
**UI/UX reasoning**: see behind things by turning, not by hunting; the pixel look holds at every angle.
**Performance budget**: ≤ 1.5 ms of CPU a frame on the render thread; GPU ≤ 8 ms a frame on the lowest supported Mac.
**Quality gates**:
- Golden frames rendered offscreen (AJ-E3513).
- A picking test (a known id under a known pixel).
- A camera-limits test.
**Evidence/validation proof**: a recording of a full turn, tilt and zoom on the release build.
**Visual testing**: the turn sequence and the tilt extremes in the gallery.
**Files to touch**: `crates/juggler-floor-render/src/` (passes, shaders, camera, pick, capture).
**Research basis**: the spike's `render.rs` (passes, `pick`, `Cam`, `pan`, `zoom_at`).
**Handover note**: `juggler-floor-render`: four passes (shadow; the scene at the pixel size with a pick id per pixel, then glass; post; overlay), picking one pixel of the id buffer, offscreen capture. The camera (`camera.rs`) and the overlay (`overlay.rs`) are pure and tested. An offscreen test draws a real office and picks a sitter's session. Left: golden frames (AJ-E3513).

### AJ-E3508 The floor logo: the tool's mark and the Account's initials | Status: IN-PROGRESS | Priority P0 | Effort M | Depends: AJ-E3505, AJ-E3507
**Goal**: Every area's floor carries its tool's logo and its Account's initials, so an area is known from across the building.
**User story**: As someone with Codex and Claude Code on two Accounts each, I find "Codex, Max · work" by the logo in its floor before I can read any sign.
**Requirements**:
1. The main window SHALL draw each needed logo:
   - the tool's mark as `ToolMark` draws it (a desktop app's own icon from `app_icons_read`, where it is the App's mark);
   - the initials of the App's Account (`accountName`), on a pill below;
   - in a ring of the App's colour.

   It SHALL render that to a 256 px PNG once and store it through a command into a registered local cache, "Floor logos" (ADR-0023). It is redrawn only when the tool's mark, the Account's name or the colour slot changes.
2. The Floor SHALL print it into the area's floor at 35% opacity, centred and large (62% of the area's short side, at most 6.5 m), blended into the floor's material, below desks and people. It carries no pick id, so clicks still reach the area.
3. Fallbacks:
   - no known Account: the App's own initials;
   - no drawable mark: the tool's name in letters;
   - no logo yet: the Floor draws the letters itself.
4. Two Apps of one tool on one Account SHALL be told apart by the ring's colour and by the App's full name on the sign. The tooltip on the floor names the App.
**UI/UX reasoning**: wayfinding by a big, consistent mark; colour never alone (letters and name too).
**Performance budget**: drawing all logos ≤ 50 ms once; nothing per frame.
**Quality gates**:
- A test of the fallbacks.
- The cache registered and clearable, and rebuilt on demand after a clear.
- No image added to the repo.
**Evidence/validation proof**: captures of the sample's floors from the default angle.
**Visual testing**: every family with its logo, from the default angle and from the building view.
**Files to touch**: `ui/src/lib/floor-logos.ts` (new), `ui/src/components/ToolMark.tsx` (export a draw-to-canvas helper), `crates/juggler-app/src/features/floor/logos.rs` (new), `crates/juggler-app/src/features/cache/` (registration), `crates/juggler-floor/src/scene/`.
**Research basis**: the maintainer's words (Source); the spike's `logo_img` and `initials`.
**Handover note**: The window draws each logo (`ui/src/lib/floor-logos.ts`: a desktop App's own icon, else the tool's allowed mark as `ToolMark` draws it, else the tool's name in letters; the Account's initials, else the App's own; a ring of the App's colour) at 256 px, once, keyed by a print (FNV-1a, the backend's `hash`), only after the Floor has opened; `floor_logo_put` keeps it in the registered "Floor logos" cache (`features/floor/logos.rs`, one file per App, the old print removed). The scene prints it at 35% into the floor (scaled to 128 px in the atlas); without one, its own letters. Tests: prints, fallbacks, keep and replace. Left: captures of every family with its logo (AJ-E3513).

### AJ-E3509 Signals from real data: every row of the table | Status: TODO | Priority P0 | Effort L | Depends: AJ-E3505, AJ-E3507
**Goal**: Everything SPEC §9.7's signal table lists is drawn from the snapshot, with its limits.
**User story**: As the boss, I see at a glance who is waiting on me, who is stuck, which session crashed, which App is overloaded, which Account is about to run out, and whether this Mac is in trouble.
**Requirements**:
1. Each row of the table SHALL be drawn as specified:
   - working, idle, waiting on you (balloon and spotlight), stuck, crashed (smoke, 10-minute limit), done (15 minutes), limit (asleep);
   - helpers (6, then `+N`);
   - App overloaded (one fire, people never burn);
   - unused (three stages);
   - Account low and out (battery, board);
   - This Mac's rack;
   - red alert (breathing at 0.5 Hz, no flash; a still tint with Motion Off or reduced);
   - tasks on the whiteboard;
   - notes on the notice board.
2. Far chips SHALL show each area's counts (`!N ?N XN`, `12%`, `OUT`, `HOT`, `UNUSED`), and the floor switcher SHALL show a waiting-on-you count per floor.
3. Motion SHALL follow Settings:
   - Follow the system;
   - Full (8 fps);
   - Calm (4 fps, no walkers);
   - Off (no animation; states still drawn).
**UI/UX reasoning**: "the main purpose of everything is to be an indicator" (D-67); never colour alone (pose, icon, word); WCAG 2.2.2 and 2.3.1.
**Performance budget**: within AJ-E3507's; a state change on screen ≤ 100 ms.
**Quality gates**:
- A table test from every state to pose, icon, word and chip.
- A test that Motion Off draws no animation frames.
**Evidence/validation proof**: a capture per signal from the replica fixture.
**Visual testing**: the signal gallery in AJ-E3513.
**Files to touch**: `crates/juggler-floor/src/signals.rs` (new), `crates/juggler-floor/src/scene/`.
**Research basis**: `rules-and-mechanics.md` §2; the spike's `Scene::frame`.
**Handover note**: —

### AJ-E3510 Doors and play: hover, clicks, keys and the fun touches | Status: IN-PROGRESS | Priority P0 | Effort M | Depends: AJ-E3509
**Goal**: Everything you can see you can point at, and everything that stands for something opens it.
**User story**: As someone who sees a person waving, I click them and land on the question in the main window.
**Requirements**:
1. Hover SHALL outline the thing in light and show a tooltip with its name and state. A hovered person SHALL look up at the camera.
2. A click SHALL open, in the main window, brought forward:
   - an area: its App;
   - a desk or person: the session (at the question or error when there is one);
   - a helper: the subagents;
   - a gauge or battery: the Account in Usage;
   - the rack: This Mac;
   - a meeting room: its pool;
   - the whiteboard: the App's tasks;
   - the notice board: Notifications;
   - the lift or directory: that floor.

   `app://open`'s `OpenRequest` SHALL gain the pages it lacks.
3. Fun props SHALL react to a click and never navigate: plants rustle, ping-pong, coffee, beanbags, the arcade, the office cat (one per building). Two people SHALL walk each floor's aisle with Motion Full.
4. Keys in the Floor window:
   - Tab and Shift-Tab between areas (the camera follows);
   - Enter opens;
   - N jumps to the next one waiting on you;
   - the camera keys (SPEC §9.7);
   - A, G and the demos only in development builds.
**UI/UX reasoning**: view and edit in place (every object has an obvious door); desktop-first navigation (one click to it).
**Performance budget**: hover outline on screen ≤ 32 ms after the pointer moves.
**Quality gates**:
- A door table test (each pick target gives its `OpenRequest`).
- A test that fun props never emit one.
**Evidence/validation proof**: a recording of each door from the Floor to the main window.
**Visual testing**: hover and tooltip captures.
**Files to touch**: `crates/juggler-floor/src/doors.rs` (new), `crates/juggler-app/src/features/notifications/mod.rs` (`OpenRequest`), `ui/src/lib/outside.ts`.
**Research basis**: the spike's `Target` and click handling.
**Handover note**: Doors go through the app's own `app://open` (`OpenRequest` gained `app`, `conversation`, `subagents`, `usage` and an `id`); `render::door` is the door table (tested; toys, the cat and floors never navigate); `ui/src/lib/outside.ts` `routeOpen` opens each in the main window (tested). Hover outline, tooltip and a hovered person looking up are in the post pass and the scene. Left: Tab/Shift-Tab between areas, Enter, N; pools' meeting rooms, the whiteboard (tasks) and notice board doors; the recording.

### AJ-E3511 Settings › The Floor, and Re-create | Status: TODO | Priority P0 | Effort S | Depends: AJ-E3503
**Goal**: The Floor's few choices live in one Settings pane, and Re-create says plainly what it will do.
**User story**: As someone who wants fewer Apps a floor, I set it to 6, and my areas move on in order.
**Requirements**:
1. Settings › The Floor SHALL hold, as `floor.*` keys:
   - Open the Floor when AI Juggler starts (off);
   - Keep it on top (off);
   - Apps per floor (4–16, default 10);
   - Motion;
   - Show unused Apps' decay (on);
   - Reset the view;
   - Re-create the office…;
   - under More, Apps hidden from the Floor.
2. Re-create SHALL ask first. It SHALL say what changes (every area's look and layout, re-placed in the same order) and what doesn't (Apps, sessions, history, the main window). It SHALL say this with an example and a link to the guide (serious choices explained).
3. A wireframe of the pane and of the Floor list (AJ-E3512) SHALL be added to `docs/wireframes/` and approved before the UI is built.
**UI/UX reasoning**: progressive disclosure (defaults that work, depth under More); serious choices explained.
**Performance budget**: the pane opens ≤ 50 ms.
**Quality gates**: the visual and a11y gates on the pane.
**Evidence/validation proof**: the approved wireframes; captures of the pane and the confirmation.
**Visual testing**: the pane in both themes, 1024 and 1280.
**Files to touch**: `ui/src/screens/settings/FloorPane.tsx` (new), `ui/src/screens/settings/panes.ts`, `docs/wireframes/`.
**Research basis**: SPEC §9.7 Settings.
**Handover note**: —

### AJ-E3512 The Floor list in the main window: the same facts, accessible | Status: TODO | Priority P0 | Effort M | Depends: AJ-E3505, AJ-E3510, AJ-E3511
**Goal**: Everything on the Floor works without seeing the canvas.
**User story**: As someone using a screen reader or only a keyboard, I get the same facts and the same doors.
**Requirements**:
1. The Floor list SHALL be a page grouped by floor and App, with:
   - the state (glyph and word);
   - session, subagents, tasks and project;
   - the Account's usage;
   - "Show on the Floor", which opens the Floor at that desk.

   It is reached from the Floor button's menu and the palette.
2. New waiting-on-you events SHALL be announced politely, once each, from the main window's live region.
3. The Floor window SHALL carry an accessibility label summarising the floor in view ("Floor 1: 6 Apps, 2 waiting on you").
**UI/UX reasoning**: accessibility is not a mode, it is the same information.
**Performance budget**: the list renders 150 rows ≤ 16 ms.
**Quality gates**: the a11y gate (axe) and keyboard traversal on the list; the visual gate.
**Evidence/validation proof**: the a11y report; a keyboard-only walk sheet.
**Visual testing**: the list, light and dark, 1024 and 1280.
**Files to touch**: `ui/src/pages/floor/` (new), `ui/src/pages/registry.ts`, `ui/src/lib/ipc/floor.ts`.
**Research basis**: the round-1 list view in `docs/research/floor/README.md`; SPEC §9.7 Accessible.
**Handover note**: —

### AJ-E3513 The Floor's gates: a gallery of golden frames, budgets and the other systems | Status: IN-PROGRESS | Priority P0 | Effort M | Depends: AJ-E3502, AJ-E3508, AJ-E3509, AJ-E3510, AJ-E3512
**Goal**: The canvas has gates as strong as the pages': its frames, its budgets and its builds are checked on every change.
**User story**: As the maintainer, a change that breaks the Floor's look or cost fails CI before I see it.
**Requirements**:
1. A Floor gallery SHALL render golden frames offscreen from the replica fixture:
   - each floor, the building, a full turn, both tilt limits;
   - every signal, every family with its logo, the model sheet, hover;
   - before and after a reflow.

   It SHALL run on software Vulkan in a Linux CI job, compared with a tolerance; changed frames are approved like baselines, with notes. A Metal smoke render SHALL run in `build-macos`.
2. The headless clock test (AJ-E3501), the determinism test (AJ-E3506) and the shared state fixture (AJ-E3505) SHALL run in `verify`.
3. The budgets in SPEC §9.7 SHALL be read on the release build with the measurement harness:
   - at rest, Full, Calm, covered, minimised, closed;
   - 5 minutes animating with memory at three marks.

   Results are committed under `docs/qa/floor/`.
4. The guide SHALL gain a page on the Floor (`docs/guide/`), and the 0.4.0 release notes SHALL describe it.
**UI/UX reasoning**: test by hand, but gate by machine first; captures checked by eye.
**Performance budget**: the gallery job ≤ 6 minutes.
**Quality gates**: every frame critiqued once (design-quality review) before it becomes a baseline.
**Evidence/validation proof**: the gallery, the harness results, the CI run.
**Visual testing**: the gallery itself.
**Files to touch**: `.github/workflows/ci.yml` (a `floor-gallery` job), `crates/juggler-floor-render/tests/`, `docs/qa/floor/` (new), `docs/guide/`, `docs/releases/0.4.0.md`.
**Research basis**: the spike's `--tour` capture and `measure/run.py`; ADR-0025 (jobs on GitHub's runners).
**Handover note**: `crates/juggler-floor-render/tests/golden.rs` draws 23 frames offscreen from a neutral sample company (made-up names, so frames can be shared; the replica's folder names come from real profiles): the building, each floor, a full turn, both tilt limits, every area (every family, every state, a hot App, unused ones, an out Account), hover, the red alert, and after a reflow. CI job `floor-gallery` (required through `verify`) runs it on software Vulkan in the visual gate's pinned Ubuntu image and uploads every frame; Linux compares against `tests/golden/` with a 2% tolerance; `build-macos` runs it as the Metal smoke render. The Linux baselines come from that job's first run, looked at by eye and committed (no local Docker build: the shared Mac's disk). Every frame was critiqued on Metal once (the hover frame reframed). The guide page is `docs/guide/the-floor.md`; `docs/releases/0.4.0.md` describes the Floor (the other 0.4 epics add their lines there as they close, and its first sentence widens with them). The budget harness (`scripts/floor_harness.py`, the `floor-harness` feature) reads every SPEC §9.7 case on the release build: `docs/qa/floor/budgets-2026-11-04.md` (12 of 15 within; the first run, `-first-run`, had 8 of 14, and its fixes are in E35's commits of 2026-11-04). Open: the two memory rows (Metal's own working memory while animating; the allocator's cache once closed), put to the maintainer with a recommendation, and the CPU at rest at the edge of its budget, since fixed: readings rebuild the snapshot at most every 30 s, and a run on a busy machine read 0.030 % (not committed: its change timings were skewed by builds running beside it; the release preflight takes the final record with nothing else running). Left: the final record, the Linux baselines.

### AJ-E3514 Walked by hand: persona walks on the release build | Status: TODO | Priority P0 | Effort S | Depends: AJ-E3513
**Goal**: The Floor is proven useful by use, not only by tests.
**User story**: As a newcomer with three subscriptions and a dozen Apps, the Floor tells me in seconds where to look.
**Requirements**:
1. A persona walk (E33's form) SHALL be done on the release build, with the replica and a live lab of 1–2 real tools in the throwaway lab:
   - open the Floor;
   - find who waits on you and answer them;
   - spot a crash and an overloaded App;
   - see a swarm of subagents;
   - find the Account running low;
   - open a new App and watch the reflow;
   - turn, tilt and reset;
   - change Apps per floor;
   - Re-create;
   - use only the list view.
2. Each step SHALL be scored for success, value, clarity and feeling. A score under 4 goes to the maintainer; a bug is fixed before 0.4.0.
3. The hand walk SHALL click every element in the Floor and its pane, and measure the response.
**UI/UX reasoning**: persona-based validation; test by hand.
**Performance budget**: as SPEC §9.7, read during the walk.
**Quality gates**: nothing under 4; no budget missed.
**Evidence/validation proof**: the walk sheet and captures under `docs/qa/floor/`.
**Visual testing**: the walk's captures, checked by eye.
**Files to touch**: `docs/qa/floor/` (walk sheet).
**Research basis**: `docs/tasks/E33-validated-by-use.md` (the walk form).
**Handover note**: —
