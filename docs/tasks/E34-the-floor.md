# E34 — The Floor: your agents as a procedural pixel office (0.5, D-65)

> Source: D-65 put the Floor in 0.5, research first. On 2026-11-03 the
> maintainer chose the Office look, with these words:
> "I like the office … imagine a real looking pixel office that is
> proceduraly generated, where each app gets a section of the office with
> each active session getting its own desk, and when zoomed in you can see
> even subtasks … procedural generated charcters are only created once …
> cached and re-used. Also there should be an option re-create everything
> somewhere in our settings."
>
> Research, design and a measured prototype are in
> [`docs/research/floor/`](../research/floor/README.md). The prototype
> (`prototype.html` + `office-engine.js`) is the reference for every task
> below: its layout rules, generator, states and budgets are what the
> build must reproduce in TypeScript inside the app, with tests.
>
> **Status 2026-11-03: the look is open again.** The maintainer then asked
> for an angled office with perspective, not a top-down view. The research
> and three look mocks are in `docs/research/floor/angled-office.html`. The
> recommendation is real 3D with three.js and a fixed angled camera, with
> the 2D renderer as the fallback.
>
> **Update 2026-11-03: native wgpu chosen.** The Floor will be a native
> wgpu window beside the webview app, not three.js in the webview. Areas use
> per-App style kits: the tool picks the family, the App id seeds the
> variant. Every area carries a sign with the tool icon, App name and mark
> colour. See [`style-kits.md`](../research/floor/style-kits.md) and the
> prototype in `spikes/floor-office/`. The scene tasks are rewritten in
> Rust for the render side when the Floor is filed for 0.5.
>
> AJ-E3401 (scene and layout), AJ-E3404 (states), AJ-E3406 (list view) and
> AJ-E3407 (budget) hold whatever look is chosen. AJ-E3402, AJ-E3403 and
> AJ-E3405 get rewritten for the chosen look before anyone claims them.

> **Order.** AJ-E3401 first; AJ-E3402 and AJ-E3403 alongside each other;
> AJ-E3404 to AJ-E3406 on top; AJ-E3407 last, on the release build.
>
> **Rules that hold for every task.**
> - Original art only, generated from code. Never copy Pixel Agents' or
>   Claude Office's sprites, layouts or fonts.
> - Generic names in every capture.
> - Colour is never the only signal.
> - Nothing is drawn while nothing changes.

### AJ-E3401 The scene model and the stable floor layout | Status: TODO | Priority P0 | Effort L | Depends: —
**Goal**: One scene model, built from the store, places every App, session, subagent, pool and room on stable desks and floors.
**User story**: As someone with ten Apps and fifty sessions, the same desk always holds the same session, and nothing jumps when others come and go.
**Requirements**:
1. `ui/src/features/floor/scene/` SHALL derive the scene from existing store selectors: Apps (with mark slot), Accounts, sessions and states, subagents, tasks, pools, notices, usage resets and This Mac. No new IPC except the inputs listed in item 6.
2. The layout SHALL be a grid of bays (4 desks each, 24 per floor). Apps claim contiguous runs in serpentine order. A session keeps its desk for life. An App that outgrows its zone takes the next free bay as an annex. A full floor opens the next floor.
3. The layout SHALL be recomputed only on Re-create or when an App is removed. Adding or ending sessions never moves anyone.
4. Reserved rooms SHALL be: server room = This Mac; meeting rooms = pools (more pools, more rooms on later floors); kitchen; archive = ended today; your office = done today; lobby = elevator, corkboard (notices), clock wall (resets).
5. Decay SHALL follow the App's state:
   - running = in use;
   - closed under a day = abandoned;
   - over a week = dust;
   - over a month, or its Account unused for a month = trash and lights off.
   Ended desks stay for a day, then free up (the maintainer may change this).
6. New, small inputs SHALL be added if the store lacks them, each with a test:
   - App last-active time;
   - Account last-used time;
   - a per-App CPU share (from the E24 process table);
   - how long This Mac has been over its load threshold.
**UI/UX reasoning**: "what is open, where, one click to it" (desktop-first navigation); stability over variety.
**Performance budget**: building the scene for 20 Apps and 150 sessions takes < 2 ms; a data change re-seats in < 1 ms.
**Quality gates**:
- Unit tests for seating, annexes, floors and decay thresholds.
- A property test: random add and end sequences never move a seated session.
**Evidence/validation proof**: the tests; a table of layouts for the 3 prototype presets that matches the prototype.
**Visual testing**: n/a.
**Files to touch**: `ui/src/features/floor/scene/` (new), the store selectors it reads, any backend field from item 6.
**Research basis**: `docs/research/floor/office-engine.js` (`Layout`, `seatSessions`, `decayOf`); the research README.
**Handover note**: —

### AJ-E3402 The generator: people, props and Re-create | Status: TODO | Priority P0 | Effort M | Depends: AJ-E3401
**Goal**: Every person and prop is generated from a seed, built once, cached and reused, and Re-create draws them all anew.
**User story**: As the boss, my staff look like a team per App and stay the same people at the same desks until I re-create the office.
**Requirements**:
1. People SHALL be assembled from parts with the shirt tinted by the App's mark hue:
   - 6 skin tones;
   - 10 hairstyles in 10 colours;
   - 6 shirt styles;
   - 6 trousers and 4 shoes;
   - glasses, headphones and beards.
   Each person is one 18-frame atlas: sit, type ×2, idle, sleep, scratch, lean, stand, wave ×2, alarm, walk ×4 directions.
2. A person SHALL be seeded by office seed + App + staff slot and built only when that App first needs the slot. Subagents are helper bots, one atlas per App hue.
3. Floors, walls, rooms and desks SHALL be baked once per floor and theme into one canvas.
4. Settings SHALL gain an Office section:
   - Motion: Follow the system / Full / Calm / Off;
   - Light: With the theme / Day / Evening;
   - "Re-create the office…", which asks first with the prototype's wording;
   - under More: show dust and trash, show helper bots, and the Apps hidden from the Floor.
5. The office seed SHALL persist in settings.
**UI/UX reasoning**: progressive disclosure; serious choices explained (Re-create says what changes and what doesn't).
**Performance budget**: a person builds in < 1 ms; a floor bakes in < 15 ms; about 22 KB per person in memory.
**Quality gates**:
- Generator snapshot tests: the same seed gives the same pixels.
- A test that Re-create changes every seeded hash and nothing in the store.
**Evidence/validation proof**: the tests; a lineup capture per theme.
**Visual testing**: a lineup and a floor capture, light and dark, critiqued.
**Files to touch**: `ui/src/features/floor/generate/` (new), the Settings page, the settings schema.
**Research basis**: `office-engine.js` (`buildCharacter`, `bakeFloor`); licences in the research README.
**Handover note**: —

### AJ-E3403 The Office renderer: zoom levels, hit-testing and doors | Status: TODO | Priority P0 | Effort L | Depends: AJ-E3401
**Goal**: A Canvas 2D renderer draws the scene on demand, at three zoom levels, and every object has a door.
**User story**: As someone glancing at the Floor, I see who works and who needs me at Floor-plan zoom, people at Desks zoom, and each monitor's tasks and subagents up close. One click opens the right page.
**Requirements**:
1. The renderer SHALL be a skin over the scene (`layout · draw · labels · hitTest`), lazy-loaded with the page, so a Plan skin can follow without changing the scene.
2. The zoom levels SHALL be:
   - **Floor plan**: icons and counts above desks, plus a one-line zone label;
   - **Desks**: people, bots and bubbles;
   - **Up close**: a cached monitor card per visible desk.
   Pixel art is drawn at integer device scale when zoomed in.
3. Needs-you bubbles SHALL stay a fixed size on screen. Edge arrows SHALL point to whoever needs you off screen. Floor tabs and the elevator SHALL show a needs-you count per floor.
4. Doors SHALL match the prototype's map table:
   - a desk opens its session (the primary door by state);
   - a zone opens the App;
   - the whiteboard opens Tasks;
   - the corkboard opens Notifications;
   - the clocks open Usage;
   - the server room opens This Mac;
   - a meeting room opens its pool;
   - the archive opens History;
   - your office opens Home.
   All open in the same window.
5. The world SHALL be drawn at its own resolution when zoomed in (one bitmap pixel per art pixel) and scaled by the compositor (`image-rendering: pixelated`, a CSS translate for the sub-pixel offset). Text, badges and bubbles SHALL be on a separate overlay redrawn only on data, camera or selection changes. The prototype measured 11% of a core at Full motion before this change.
6. Frames SHALL be drawn only on a change. A ticker (10 fps; 4 in Calm) SHALL run only while something animated is on screen, the page is visible and the canvas is in view. It stops at Floor-plan zoom and with Motion off.
**UI/UX reasoning**: view and edit in place; desktop-first navigation; instant (the cached scene shows at once).
**Performance budget**: see AJ-E3407.
**Quality gates**:
- Hit-test unit tests.
- A test that no frame is drawn at rest, and none while hidden.
**Evidence/validation proof**: per-zoom captures; a frame counter log at rest and while animating.
**Visual testing**: three zooms × two themes × three window sizes, critiqued against the design-quality review.
**Files to touch**: `ui/src/features/floor/render/` (new), the Floor page and its route, the header entry.
**Research basis**: `office-engine.js` (`FloorView`), `prototype.html`.
**Handover note**: —

### AJ-E3404 States, decay, fire and red alert from real data | Status: TODO | Priority P0 | Effort M | Depends: AJ-E3402, AJ-E3403
**Goal**: Every rail state and App condition has its pose and props, driven by real data.
**User story**: As the boss, someone standing up waving means a question for me, a sleeper means a limit, and a fire means an App is overloaded.
**Requirements**:
1. The rail states SHALL map to poses:
   - Working: typing;
   - Asks you: stands, faces you and waves;
   - Stopped with an error: alarm;
   - Limit reached: asleep;
   - Probably stuck: scratching their head;
   - Done: leaning back, with a review paper;
   - Idle: still;
   - Ended: an empty, tucked-in desk.
   Each pose also has its glyph and word.
2. Decay (abandoned, dust, trash with lights off) SHALL follow AJ-E3401's thresholds.
3. App overload SHALL show a desk fire at its busiest desks, the "Overloaded" label and a notice. Sustained This Mac overload (5 minutes over 90% by default) SHALL show red alert: a steady wash breathing at 0.5 Hz, beacons and a banner with the number. There is never a flash (WCAG 2.3.1), and with motion reduced it is a still tint.
4. Subagents SHALL show as up to three bots beside the desk (amber when waiting), with the count on the badge. Tasks SHALL show on the zone whiteboard and on the monitor card.
**UI/UX reasoning**: never colour alone; honest states (only what the store knows).
**Performance budget**: within AJ-E3407.
**Quality gates**: a table test, from state to pose to glyph to word, covering every state.
**Evidence/validation proof**: a capture per state (light and dark), from the replica fixture.
**Visual testing**: the state gallery, critiqued.
**Files to touch**: `ui/src/features/floor/render/`, `ui/src/features/floor/scene/`.
**Research basis**: the design page §01 and §04.
**Handover note**: —

### AJ-E3405 Ambient life and event scenes | Status: TODO | Priority P1 | Effort M | Depends: AJ-E3404
**Goal**: A few people move, each for a reason.
**User story**: As someone glancing at the Floor, motion tells me something happened, never just decoration for its own sake.
**Requirements**:
1. Ambient movement SHALL be:
   - idle people fetching coffee or water;
   - the mail carrier pinning new notices;
   - a runner carrying a pool hand-off between members' desks;
   - the office cat.
   At most 3 walkers at once, on grid paths, and only with Motion Full.
2. Event scenes SHALL be:
   - a new session's hire walking from the elevator to their desk;
   - an ended session packing a box;
   - a switch (A → B) carrying a box to the new App's desk;
   - a crash and restart fixed by a repair bot;
   - an update parcel at reception.
3. Each walk SHALL be skipped when its floor is not on screen, so a scene never delays data.
**UI/UX reasoning**: ambient awareness without noise; WCAG 2.2.2 (one Motion control pauses all of it).
**Performance budget**: walkers add < 0.3 ms per frame.
**Quality gates**: path tests on every generated floor (every desk reaches the elevator).
**Evidence/validation proof**: short screen recordings, one per scene.
**Visual testing**: captured mid-walk.
**Files to touch**: `ui/src/features/floor/life/` (new).
**Research basis**: `office-engine.js` (`walk`, `poolHandOff`, `noticeArrived`).
**Handover note**: —

### AJ-E3406 The list view, keyboard and announcements | Status: TODO | Priority P0 | Effort S | Depends: AJ-E3403
**Goal**: Everything on the Floor works without seeing the canvas.
**User story**: As someone using a screen reader or only a keyboard, I get the same facts and the same doors.
**Requirements**:
1. The List view SHALL be a table grouped by floor and App with: state (glyph + word), session, subagents, tasks, project, and "Show on the Floor".
2. The canvas SHALL be focusable. Arrows pan, + and − zoom, F fits, N jumps to the next who needs you. The Needs-you list is plain buttons ordered by severity.
3. New needs-you events SHALL be announced politely, once each. The canvas label SHALL summarise the floor.
**UI/UX reasoning**: accessibility is not a mode; it is the same information.
**Performance budget**: the list renders 150 rows in < 16 ms.
**Quality gates**: the a11y gate (axe) on the Floor page in both views.
**Evidence/validation proof**: the a11y report; a keyboard-only walk sheet.
**Visual testing**: list view captures, light and dark.
**Files to touch**: `ui/src/features/floor/` page and list.
**Research basis**: the design page §06.
**Handover note**: —

### AJ-E3407 The budget proved in the real app, then walked by hand | Status: TODO | Priority P0 | Effort S | Depends: AJ-E3404, AJ-E3405, AJ-E3406
**Goal**: The Floor costs what the prototype measured, in the release build on the maintainer's kind of setup.
**User story**: As someone with the Floor open all day, I can't find it in Activity Monitor.
**Requirements**:
1. Each budget SHALL be read for 60 s in Activity Monitor (WebContent + GPU + app) on the release build:
   - at rest: 0 frames, no measurable added CPU;
   - live data at Floor-plan zoom: one frame per change, < 3 ms each;
   - animating at Desks zoom: ≤ 10 fps, < 4 ms per frame, and less than 3 percentage points of one core above the app without the Floor;
   - hidden or scrolled away: 0 frames.
2. Before this check, `docs/research/floor/measure/measure.cjs` SHALL be re-run on the built page in WebKit and Chromium, with results committed next to the prototype's.
3. A persona walk SHALL be done: open the Floor, find who needs you, answer them, re-create the office, switch floors, and use the list view only. It is scored as E33 does.
**UI/UX reasoning**: always on; lazy-load everywhere; persona-based validation.
**Performance budget**: as above.
**Quality gates**: nothing under 4 in the walk; no budget missed.
**Evidence/validation proof**: `docs/qa/floor/` with the readings, the results JSON and the walk sheet.
**Visual testing**: the walk's captures.
**Files to touch**: `docs/qa/floor/` (new).
**Research basis**: the measured table in `docs/research/floor/README.md`.
**Handover note**: —
