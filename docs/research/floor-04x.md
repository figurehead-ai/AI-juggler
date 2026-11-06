# The Floor in 0.4.x — research for the maintainer's 0.4.0 findings

> Status 2026-11-06. Source: the maintainer's first test of the Floor on
> 0.4.0 (nine findings, three screenshots). Rulings: the bugs (the hand
> cursor, the floor tabs, the floor that appeared when zooming out, the
> desktop App names, Apps only seen running, the long leader lines) ship
> in **0.4.1**; the design items below ship in **0.4.2**, as E35 tasks
> AJ-E3515…AJ-E3521. Design calls are decided by the goal (the Floor is an
> indicator first: where work is busy, waiting on you, stuck, crashing or
> overloaded) and reported here; the visual direction is shown, not
> described, in the proposal artifact (§10). Budgets: SPEC §9.7 and D-79,
> both looks (D-80).

Contents: 1 Labels by zoom · 2 One floor at a time, and the building ·
3 Hover cards · 4 Inactive Apps · 5 The Floor menu · 6 Day and night ·
7 Light, shadow, material and colour · 8 What 0.4.1 fixed · 9 Costs in one
table · 10 The proposal · 11 Sources.

---

## 1. Labels by zoom (finding 3)

**The problem, measured on the maintainer's screenshots.** Zoomed out, the
area chips are 7-pixel glyphs scaled by the pixel unit, so they shrink to
unreadable with the scene. Every App gets a chip whether or not anything
needs you (16 chips over two floors in the building view), and a chip
that can't sit over its area slides up to ten rows and two chip widths
away on a stick; in the building view the sticks crossed the other floor.
Calm Apps (UNUSED, OUT) take the same room as an App with a question.

**How good games and tools do it.**

- **Maps (Google Maps, Mapbox GL, Apple Maps)** place labels by
  *priority*, then by *collision*: each label has a rank; the placer walks
  them highest first and drops any that would overlap one already placed.
  Text stays at a fixed screen size; what changes with zoom is *which*
  labels exist. Nearby points merge into a cluster with a count
  (Supercluster, Mapbox's point clustering), which splits as you zoom in.
  Fading in and out over ~300 ms hides the switch.
- **City builders (Cities: Skylines, Two Point Hospital)** show district
  and building names only when zoomed out, room and person detail only
  close, and *problems* (a fire, a sick patient, a broken machine) as
  icons that survive every zoom: the notification is never decluttered
  away, the name is.
- **RimWorld** shows colonist name labels at a fixed pixel size and drops
  them past a zoom threshold; alerts move to a list at the screen's edge
  that jumps the camera to the colonist.
- **Factorio's Alt mode** turns on a dense overlay on demand rather than
  by default: depth on demand.
- **RTS games (StarCraft II, Age of Empires IV)** use icon pings that keep
  a fixed screen size and stack into one ping with a count when close.

**Options.**

| | Option | For | Against |
|---|---|---|---|
| A | Keep chips, scale text with zoom | Least work | Unreadable far out; the sea of labels stays |
| B | Fixed-size chips for every App, collision-dropped | Readable | Drops the wrong ones (an App with a question can lose to a calm one) |
| C | **Three zoom bands; attention-only far out, clustered; flags on short poles** | The indicator reads at every zoom; never a sea | More code in the overlay (pure, testable) |
| D | A side list of alerts only | Always readable | Takes the eye off the office; duplicates the main window's Floor list |

**Pick: C.**

- **The band comes from how big a desk is on screen** (device pixels per
  metre at the area), not from the camera distance, so a large window and
  a small one agree. Bands switch with 15% hysteresis so a pinch at the
  boundary doesn't flicker.
- **Close** (a desk ≥ 28 px): the scene speaks for itself. Balloons
  over desks (as today), the area's hanging sign in the scene, and the
  hover card (§3). No overlay chips.
- **Middle** (a desk 10–28 px): **compact flags pinned to their desk**
  for sessions that need you, on a pole one flag tall, never moved
  aside: state icon + one word (`Asks`, `Stuck`, `Crashed`, `Done`). Each
  area gets one name flag at its way in (the App's colour, its short
  name). Calm sessions (working, idle, asleep) get no flag; their pose
  says it.
- **Far** (a desk < 10 px, and the building): **attention only**. One flag
  per area that needs you, fixed-size text (11 pt, the font at an integer
  scale), its App's colour on the pole: `! 2 asks · Claude app · work`.
  Flags that would overlap merge into one cluster flag (`3 waiting`)
  pinned at their centre, which splits as you zoom in. Calm Apps show no
  flag at all; OUT and low usage show as a battery mark on the flag of an
  App that also needs you, else only in the lobby's usage board and the
  hover card.
- **Priority** (placed first, never dropped for a calmer one): Crashed >
  Asks > Stuck > Hot > Done > Out. A flag never moves more than its pole
  from its anchor (0.4.1's REACH already caps it at three rows); one that
  finds no place merges into its neighbour's cluster instead of
  vanishing, so a count is never lost.
- **Colour is never the only cue**: icon + word + colour on every flag.

**Cost.** CPU: the placer is O(n log n) over at most ~160 anchors (sessions
needing you plus areas), well under 0.1 ms; the overlay quads drop from
up to ~40 chips to typically under 10. GPU: unchanged (overlay quads). No
memory change.

---

## 2. One floor at a time, and the building (finding 4)

**The problem.** On a floor, zooming out past 1.55× the floor's framing
switched to the stacked building and drew every floor: the floor below
appeared from nowhere. In the building view the floors overlap at the
default angle, so neither reads.

**How games do it.**

- **The Sims** shows one storey at a time with up/down buttons and Page
  Up/Down; storeys above are hidden, below are dimmed context. The camera
  never decides the storey for you.
- **SimTower, Tiny Tower, Fallout Shelter** draw the whole building
  side-on (an elevation): each floor a strip with its tenants, lit or
  dark; tapping a floor goes into it. A count or icon per floor says
  where something needs you.
- **Project Hospital, Prison Architect** use a floor stepper in a corner
  with the floor's name.
- **Lifts in games** (Two Point's floors, Hitman's map) show the current
  floor highlighted in a short vertical list, with arrows.

**Options.**

| | Option | For | Against |
|---|---|---|---|
| A | Keep the stacked 3D building, spread the floors apart | Small change | Still overlapping at most angles; labels fight |
| B | **One floor at a time + a side-on tower for the building** | Each view reads; the tower shows tenants and waiting counts per floor at a glance | A second drawing of the office (cheap boxes) |
| C | Only floors, no building | Simplest | Loses the "whole company at a glance" overview SPEC §9.7 promises |

**Pick: B.**

- **On a floor**: zoom stays on it (done in 0.4.1). A **lift stepper** on
  the right edge, vertically centred: `▲ Floor 3 · 2 waiting`, the current
  floor's number, `▼ Floor 1`; the arrows are hidden at the top and the
  bottom. Page Up / Page Down and the number keys do the same. Changing
  floor tweens the camera up or down one storey (0.45 s, or a cut with
  Reduce Motion) so the move reads as a lift.
- **The tabs** stay at the top left as the one place that names every
  view (Building · Floor 2 · Floor 1), each with its waiting count.
- **The building** becomes a **side-on tower**, drawn in the scene with an
  orthographic camera: each floor a strip, its areas as lit rooms in their
  App's colour with the tool's mark, dark when inactive (§4); at the
  strip's right a waiting count (`! 3`) and at its left the floor number.
  The lobby is the ground floor. Hover shows the floor's summary; a click
  enters that floor with the camera dropping into it.
- No auto-switch between the floor and the building by zoom; the tower
  is a choice (its tab, B or 0), not a zoom accident.

**Cost.** The tower is ~10 boxes and a mark per App: well under 1 ms to
build and draw; no new textures (the marks are in the atlas already).
Only the floor in view is drawn on a floor, so a floor view is *cheaper*
than 0.4.0's zoomed-out case that drew every floor.

---

## 3. Hover cards (finding 6)

**Want.** Hover a person or a terminal: a speech-bubble card that says
what they want or are doing, enough to decide whether to act, with "Open
in AI Juggler" and "Open in <the tool's app>".

**How others do it.** RimWorld's inspect pane and The Sims' hover show
*state and the current want* in one line. Two Point Hospital's thought
bubbles say the patient's need in an icon and a word. GitHub's and
Linear's hover cards open after a short delay (~300–500 ms), stay while
the pointer travels into them, and hold one or two actions; Figma's use a
"safe triangle" so moving diagonally to a button doesn't close the card.

**Pick.**

- **Content**, top to bottom: the state as icon + word in its colour
  (`! Asks you`), the session title, then one line that matters: the
  question it asks (the first ~120 characters of the pending question or
  permission), else what it's working on (its latest step), else how long
  it's been idle; the project and subagent count as a quiet line.
- **Actions**: `Open in AI Juggler` (the session, at the question; what a
  click on the person already does) and `Open in <Claude app / Codex /
  Terminal>` (the session's own app, through the adapter's existing "Go to
  it" path; shown only when the adapter can open it there).
- **Behaviour**: appears 250 ms after the pointer rests on a person,
  desk or screen; points at them with a tail (the speech bubble); stays
  while the pointer is on its subject, its card or the corridor between
  them, and 300 ms after; Esc closes it. Drawn in the overlay at fixed
  size; buttons show the hand cursor.
- **Privacy**: an excerpt only, never stored by the Floor; in captures
  the gallery's made-up company.

**Data.** `FloorSession` gains `ask` (the pending question's text, from
the same source the main window shows it) and `doing` (the last step). The
feed already reads both rows; the snapshot carries at most 160 characters
each.

**Cost.** CPU negligible (one card, at most ~40 quads); the snapshot grows
by ~300 bytes a session needing you.

---

## 4. Inactive Apps: lights off, dust and covers (finding 8)

**Today.** SPEC §9.7's "App unused" row: under a day dim with chairs
pushed out, over a week dust, boxes and wilted plants, over a month
lights off. In 0.4.0 the decay is three steps and hard to read; nothing
says "off" as clearly as a dark room.

**References.** Fallout Shelter's unpowered rooms go dark at once (the
clearest "not working" in the genre). Dust sheets over furniture are the
universal sign of a closed house (film and games alike: Gone Home,
Hitman's mansions, The Sims' "for sale" lots). The Sims and Two Point
show dirt as a growing overlay rather than a switch.

**Pick: continuous, by time since the App was last active.**

| Since last active | What you see |
|---|---|
| Open, no session working | Lights on, normal |
| App closed, under 1 hour | Desk lamps and screens off; the area's light at 80% |
| 1 hour – 1 day | Area light at 55%; chairs pushed in |
| 1 – 7 days | Darker (40%); a dust film on top faces, growing with days |
| 7 – 30 days | Dust sheets over desks and screens; plants droop |
| Over 30 days | All covered, boxes by the door; the area nearly dark (25%) but its logo and sign still readable |

- **Light**: a per-area light multiplier (the room's own lamps and the
  ambient term inside its cell), so a dark area sits beside a lit one.
- **Dust**: a world-space noise film on upward faces, amount per instance
  (0–1), in the scene shader; no texture.
- **Covers**: a draped-cloth mesh (a box with a soft skirt and three folds)
  that replaces the desk and screen model; fewer triangles than the desk.
- Waking: when the App runs again, the lights come on first (the room
  "wakes" in 0.6 s), covers disappear on the next snapshot.
- The setting "Show unused Apps' decay" (on) stays the off switch.

**Cost.** A float per instance (already a `misc` slot) and a few shader
instructions; covers lower the triangle count. No CPU budget effect.

---

## 5. The Floor menu, top right (finding 9)

**Pick.** A small button at the top right of the Floor (a funnel icon
and "View"), opening a **native menu** (an `NSMenu` through Tauri's menu
API, so it is keyboard- and VoiceOver-accessible and costs no overlay
work):

- **Show**: All Apps · Active Apps only (inactive ones hidden).
- **Arrange**: Kept places (the default; the saved order) · Active Apps
  packed together (a view-only re-layout: active Apps first-fit from
  the first cell, nothing saved, Kept places returns them).
- **Labels**: Smart (§1) · Attention only · All names.
- **Light**: Follow my clock (§6) · Always day · Always night.
- **Look**: Pixel art · Full resolution (D-80, the same key as Settings).
- Settings › The Floor… (the main window's pane).

Each choice is remembered in the `floor.*` keys, shown with a check mark,
and applies at once. "Packed" never writes the office: positions stay
stable (SPEC §9.7's stability rule) and come back exactly.

**Cost.** None while closed; a re-pack is the existing layout pass
(≤ 20 ms reflow budget).

---

## 6. Day and night from the Mac's own clock (finding 7)

**References.** Animal Crossing and Pokémon Gold/Silver run on the
player's real clock; macOS's dynamic wallpapers follow the sun's real
position from the Mac's location; f.lux uses sunrise and sunset, not
fixed hours. A real sun feels alive; fixed hours feel wrong half the year.

**Pick.**

- **Where the sun is**, without asking for Location: the Mac's time zone
  (the `/etc/localtime` link, e.g. `Europe/Sarajevo`) and that zone's
  coordinates from the system's own `zone.tab` (`/usr/share/zoneinfo`),
  read at run time, nothing bundled. Accurate to a zone's width: sunrise
  within ~20 minutes, which is enough for light.
- **When**: the NOAA solar position equations (declination, equation of
  time, hour angle) give the sun's elevation and azimuth for any minute:
  a few dozen floating-point operations.
- **What changes**, by the sun's elevation: day (sun > 6°) warm white
  sun, blue sky; golden hour (0–6°) low orange sun, long shadows; civil
  twilight (0 to −6°) violet sky, lamps on; night (< −6°) dark blue
  ambient, desk lamps and screens carry the room, windows lit from
  inside, stars in the sky. The sun's direction in the scene follows its
  azimuth, so shadows swing through the day.
- **Redraws**: the light steps every 10 minutes (one frame); nothing
  animates for it, so the at-rest budget (0 frames, < 0.1% of a core)
  holds. Covered or closed, nothing runs.
- **Off switch**: Light › Follow my clock / Always day / Always night (§5,
  and Settings › The Floor).
- **Readability**: signals keep their contrast at night (balloons and
  flags are lit, the overlay is unchanged); night lowers the scene, not
  the signals.

**Cost.** CPU: microseconds every 10 minutes. GPU: none beyond today's
lights (the night uses the existing point lights).

---

## 7. Light, shadow, material and colour (finding 5)

**Today** (`crates/juggler-floor-render/src/shaders.rs`): one sun with a
single-tap shadow (Pixel art) or five taps (Full), the sun quantised in
three bands, a hemisphere ambient, point lights, outlines from depth and
pick ids, a vignette. No ambient occlusion: furniture floats, corners
are flat. Textures are small flat tiles; colours sit at similar
saturation and value, so the signals don't stand out from the furniture
(the "real colour hierarchy" rule).

**How good-looking low-poly and pixel-3D games get their look.**
Townscaper, A Short Hike, Islanders, Tunic, Monument Valley, Death's
Door, and the pixel-3D work of t3ssel8r (a lit low-poly scene rendered
at low resolution with a pixel-snapped camera):

- **Ambient occlusion** does most of the "grounded" feel: soft dark at
  contacts and corners. At low internal resolution SSAO is cheap.
- **Soft shadows** from filtered shadow maps (PCF with a rotated Poisson
  disk) and a shadow map fitted tightly to what's in view.
- **Toon-banded light** kept for the painted look, but with a warm key
  and cool fill (colour temperature contrast), a rim light on people.
- **Restrained palettes**: environment desaturated and mid-value,
  characters and signals saturated; a grade (tone curve, split toning)
  per time of day.
- **Texture detail at the pixel scale**: noise-dithered wood grain,
  carpet weave, terrazzo chips, with 2–3 tones per material.
- **Outlines** from depth and normals (t3ssel8r: darker on silhouettes,
  lighter on creases).

**Options and the pick.**

| Technique | Pick | Why | Cost (GPU per frame on an M-series, CPU unchanged) |
|---|---|---|---|
| SSAO at the scene's pixel size, 8 samples, 4×4 blur | **Yes, both looks** | Biggest visual gain: furniture sits, corners deepen | Pixel art (~640×400): ~0.1 ms; Full (2556×1596): ~0.8 ms; one R8 target (0.25 MB / 4 MB) |
| Contact shadow blobs under people and furniture (baked into meshes) | **Yes** | Grounding even where SSAO is thin | Zero at run time |
| Shadow map fitted to the floor in view; PCF 8–12 taps, rotated Poisson | **Yes** | Soft, stable edges; no shimmering in Pixel art | +~0.1 ms; same 2048² map |
| Warm key / cool fill, rim light on people | **Yes** | Depth and readability of people | A few ALU ops |
| Normal + depth outlines (crease lines lighter) | **Yes** | Crisper forms | Post pass, ~0.05 ms |
| Per-material specular (banded) on floors, desks, glass | **Yes, light** | Polished floors read as floors | A few ALU ops |
| Colour grade per theme (tone curve + split tone), signals exempt | **Yes** | The colour hierarchy: scene recedes, signals pop | Post pass, negligible |
| Richer procedural textures (atlas 1024² → 2048²) | **Yes** | Wood, carpet, terrazzo read as materials | +12 MB GPU memory once; inside D-79 |
| Real-time global illumination, ray tracing | No | Far over budget, invisible at pixel scale | — |
| Bloom | Only on lit screens and lamps at night, at scene res | Night glow | ~0.1 ms |

- **Themes**: the grade and the palette's ambient move with day and night
  (§6); the families' kits keep their own colours.
- **Budgets**: the CPU budgets (≤ 2% of a core animating, ≤ 1.5 ms CPU a
  frame) are unaffected: all of this is GPU work in passes that already
  run. Memory: +0.25 MB (Pixel art) / +4 MB (Full) for the AO target and
  +12 MB for the larger atlas; D-79's 250 MB animating budget holds with
  room. Each will be re-measured with `scripts/floor_harness.py` on both
  looks before the rework lands, and the record says the numbers.
- **The order of work**: show the before/after (§10) first; build the
  rework only after the maintainer has seen it.

---

## 8. What 0.4.1 fixed (findings 1, 2, part of 4; bugs a, b, c)

- **The hand cursor** over anything a click acts on: doors, toys, floors,
  the floor tabs (`Driver::hand`, the window's cursor through Tauri).
- **The floor tabs didn't answer**: clicks were measured from the
  window's content-layout rect, not the view the Floor draws into, so they
  landed about a title bar's height below the tabs (found on the hand
  walk). Events are now read in the drawing view's own coordinates. And a
  press that wobbles a point or two on a trackpad was read as a drag; it
  now has a 4-point slop (`juggler-window-events::past_slop`).
- **One Window › The Floor made two windows**: the app menu's handler and
  the tray's both get the press; a press seen twice now toggles once.
- **One floor at a time**: zooming out on a floor stays on it, out to
  1.6× its framing; the building is a choice (its tab, B or 0), never a
  zoom accident.
- **(a)** A desktop App is named as the rail names it: "Claude app ·
  b-work", from the places book's word, never "Claude Code · …".
- **(b)** The Floor's Apps are the rail's (D-84): the user's own Apps
  (`App::is_users`), so an app only seen running is never an area.
- **(c)** A chip sits at most three rows (or half its width) from its
  area. In the building view only chips that need you are drawn, and one
  with no room near its area folds into its nearest same-floor neighbour
  as "+N"; nothing hangs on a long line.

---

## 9. Costs in one table

| Item | CPU | GPU | Memory | Budget verdict |
|---|---|---|---|---|
| Labels by zoom | < 0.1 ms per frame drawn | fewer quads | — | holds |
| Lift stepper + tower | < 1 ms to build | ~10 boxes | — | holds; floor views draw one floor |
| Hover card | negligible | ≤ 40 quads | ~300 B per waiting session | holds |
| Inactive decay | — | a few ops per pixel | — | holds |
| Floor menu | — | — | — | holds |
| Day and night | µs every 10 min | existing lights | — | holds; 0 frames at rest |
| Light and material rework | — | +0.4 ms (Pixel art) / +1.2 ms (Full) | +12 MB / +16 MB | holds; re-measured before it lands |

---

## 10. The proposal

The visual proposal (label bands at three zooms, the hover card, the lift
stepper and the tower, lighting before and after, day and night,
inactive and dusty areas) is drawn from the real renderer's offscreen
frames of the gallery's made-up company, with the overlay mocked over
them where it isn't built yet. It is published privately for the
maintainer's review before the big visual rework is built:
<https://claude.ai/artifact/1JYw8ArLXRii6Y1QbFYuCE>. The frames come from
`AJ_FLOOR_PROPOSAL=<folder> cargo test -p juggler-floor-render --test
golden -- --ignored proposal`; the reworked look is already in the
renderer behind `Look { rework: true }`, off by default.

## 12. The second pass: materials, objects and people (maintainer's review)

The maintainer asked, on seeing the proposal, whether the textures of all
objects and people could be of higher quality, still all generated, and
whether v1 was the best we can get. **It wasn't.** v1 drew every object
and person as flat colour on boxes and balls; only floors, walls and
signs had textures (24 px a metre). The second pass (v2) is built behind
a switch (`Look::materials`, scene input `detail`), nothing bundled:

- **Materials** (`juggler_floor::materials`): 16 tileable maps generated
  at run time from periodic gradient, fractal and cellular noise: wood
  (flat-sawn rings, fibres, pores), plain-weave fabric, loop-pile carpet,
  brushed metal, satin plastic, leaf (midrib, veins, areoles), skin
  (pores, mottling), hair (strands, clumps), cotton with folds, denim
  twill, pebbled leather, glaze, plaster, rubber, paper, soil. Each is 256²
  RGBA8 (colour multiplier, roughness, normal x, y) with every mip level.
  Normals come from the generated height. Mips fold lost normal variance
  into roughness (Toksvig), so a bumpy surface turns matt at a distance
  instead of sparkling. They're made on 16 threads in about 10 ms the first
  time v2 draws, into one texture array (5.3 MB). A part keeps its own
  colour; the map adds the surface.
- **Shading**: triplanar in the part's own metric space (a texture sticks
  to what it's on, and keeps its size however the part is scaled), with
  normals blended by the UDN method. GGX highlights from roughness, the sky
  reflected by Fresnel, a cool fill light, sheen on cloth, Kajiya-Kay
  highlights along hair, light wrapping round skin and leaves. The sun's
  bands are softened (more of them kept in Pixel art).
- **Geometry**: five smooth meshes (sphere, capsule, rounded box, smooth
  cylinder, torso). Desks, chairs, props, pots and plants are tagged with
  their materials. Floors and walls take theirs from the kit
  (`FloorMat::mat`, `WallFinish::mat`).
- **People** (`models::person_v2`): smooth proportioned bodies, hands with
  thumbs, ears, a nose, shoes with soles, hair as strands, glasses and
  beards, and a **face drawn by the shader**. Eyes (iris, pupil,
  catch-light, lashes), brows and mouth come from the state
  (`models::expression`): focused working, hopeful when asking, worried
  when stuck, alarmed when crashed, happy shut eyes when done, asleep
  when out of usage. They blink, breathe and sway a little, only while
  the Floor already animates (0 frames at rest).
- **Pixel art** keeps its look; the maps read as pixel texture. A finer
  pixel (`Look::fine`: 2 device pixels on Retina instead of 3, never under
  2) keeps more detail for +0.04 ms a frame.

**Measured** (this Mac, the gallery company; the test
`the_second_pass_frames` writes the frames and `costs.json`):

| | v1 | v2 |
|---|---|---|
| Maps (once) | — | 10 ms, 5.3 MB |
| Scene build | 5.8 ms | 5.9 ms |
| Moving parts on floor 1 | 1,681 | 1,942 |
| CPU to build a frame's moving parts | 0.050 ms | 0.057 ms |
| Frame, Retina, Pixel art / finer / Full | 0.79 / — / 1.24 ms | 0.89 / 0.93 / 1.45 ms |
| Frame, 3440 × 1410, Pixel art / Full | 0.84 / 1.43 ms | 1.01 / 1.60 ms |

Budgets (SPEC §9.7, D-79): Pixel art holds every one. Full resolution
was already 4 MB over memory open and still; the maps make that about
9 MB over. Either the budget moves or the atlas is trimmed (preferred: it
is larger than it needs to be). The harness re-measures both looks on the
release build before v2 becomes the default.

**Honest limits.** At the default zoom a person is about 30 pixels tall,
so most material detail averages out. v2 pays off up close and in Full
resolution. The ceiling of this way of building (parts from ~19
generated shapes, one shadow map, no bounce light) is about v2 after
one more tuning round. What gets past it, still generated:

| Limit | Past it | Cost |
|---|---|---|
| Rigid parts: seams at joints, mitten hands | One skinned body per person from a parametric SDF template, fingers, hair cards | 2–3 weeks; GPU +0.2 ms, ~2 MB |
| Boxy furniture | SDF modelling meshed at build, cached | 1–2 weeks; +20–40 ms first build |
| No bounce light | Light probes or screen-space GI | ~1 week; GPU +1–3 ms |
| Maps blur right up close | 512² maps, or made on the GPU per zoom | +16 MB, or ~1 week |

Beyond these is hand-made art (sculpted characters, photographed
materials), the only way to a film or AAA look. It breaks the
nothing-bundled rule and adds tens of MB, licences and ongoing art work.
I don't recommend it.

## 11. Sources

- Mapbox GL JS symbol placement and collision; Supercluster (point
  clustering), Mapbox docs.
- NOAA Global Monitoring Laboratory, "General Solar Position
  Calculations" (the equations used in §6).
- IANA time zone database, `zone.tab` (each zone's coordinates).
- John Chapman, "SSAO tutorial"; Crytek's SSAO (Mittring 2007);
  "Rendering Antialiased Shadows with Depth Maps" (PCF, Reeves 1987).
- t3ssel8r's pixel-art 3D renderer talks and devlogs (low-res lit 3D,
  pixel-snapped camera, depth/normal outlines).
- Games as references only (no assets, looks or levels copied): The Sims,
  SimTower, Tiny Tower, Fallout Shelter, Two Point Hospital, Cities:
  Skylines, RimWorld, Factorio, Townscaper, A Short Hike, Animal Crossing.
