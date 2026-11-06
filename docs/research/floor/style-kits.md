# The Floor — native wgpu and per-App style kits

> Status 2026-11-03. The maintainer chose **native wgpu** for the Floor
> after the measured spike (`spikes/floor-spike/` on the branch `proto/floor-office-2`, `floor-spike-results.html`).
> They then asked for real renderings that are "a lot more varied and
> intricate", under four rules:
> 1. each App's area looks distinct, and an App always favours its own look;
> 2. each area has a flag or sign that names its App;
> 3. many things are procedurally generated;
> 4. the office looks nice and feels good.
>
> The first prototype is `spikes/floor-office/` on the branch `proto/floor-office-2`. This note records the
> rendering decision and the style-kit design so the E35 tasks can be
> rewritten from it when the Floor is filed for 0.5.

## Decision: a native wgpu window

- **What.** The Floor is its own native window with no webview, created
  with Tauri's `unstable` `WindowBuilder` and drawn with wgpu (Metal on
  macOS). The main window stays the webview app. Clicking an area or a
  person opens that App or session in the main window.
- **Why.** The spike measured both approaches on the same scene:
  - native wgpu used about 1% of one core at 10 fps;
  - the Canvas 2D webview cost several times that, mostly in the
    WebContent and GPU processes;
  - native drew nothing while covered or minimised.
- **What it costs.**
  - Tauri gives a window without a webview no mouse, keyboard or
    occlusion events. The prototype reads them from AppKit directly (an
    `NSEvent` local monitor, and `occlusionState` polled at 2 Hz).
    Windows needs a window-procedure hook and Linux needs GTK signals:
    one adapter per OS behind the same `Msg` enum.
  - Text is not free. The prototype has a 5×7 pixel font for signs; the
    real build needs a proper glyph atlas (still pixel-aligned) and must
    keep labels short.
  - Accessibility cannot come from the canvas. The main window carries an
    accessible list of the same areas and desks (the E35 list view).
- **Draw policy (kept from the spike).**
  - Frames are drawn on demand only.
  - A tick runs only while animated content is in view.
  - Nothing is drawn while the window is covered or minimised.
  - Floors and walls are baked once into a background texture; people and
    props are depth-sorted instanced quads over it, with a screen-space
    overlay for chips and labels.

> **Round 2 (2026-11-03).**
> - The Floor is now low-poly 3D drawn as pixel art, not painted 2D, so it can turn and tilt.
> - Each family has a set of layout types: Workshop pods, pinwheels or scatter; Studio tables,
>   scatter or radial; Industrial benches, diagonal rows or pods; Garage scatter or pinwheels;
>   Campus pinwheels, radial or tables; Lab pods or diagonal rows.
> - The App's layout type and seed are chosen once and saved.
>
> The kits also gained more variety: 6 desk models, 5 chair types, 8 plants in 5 pots, 6 divider
> styles and 7 nooks. See [`rules-and-mechanics.md`](rules-and-mechanics.md) for the layout and
> signal rules.

## Style kits: one look per App, chosen deterministically

Real offices give each team a neighbourhood with its own character. That
helps people own the space, and it helps everyone else find their way
(Steelcase; Gensler; Herman Miller's Living Office). Procedural games
learned that output which is mathematically unique can still look
identical (Kate Compton's "10,000 bowls of oatmeal"). So the kit varies the
big things, and the props carry the small variation (the Townscaper
write-up).

**A kit is a bundle.** `kit_for(tool, app_id, mark_slot)` returns:
- the floor material and wall finish;
- the desk family and top colour, the chair family and colour;
- the layout (pods, bench, clusters, studio, tables) and screens;
- the plant species and greenery level;
- the decor set and the nook;
- the light warmth, the window count and the rug colour;
- the sign style;
- the App's accent colour, from `MARK_COLOURS[slot]` (the same OKLCH hues
  as the App mark in the main window).

**The tool picks the family, the App picks the variant.** The random
stream is seeded with `kit|<tool>|<app id>`. So the same App always gets
the same kit. Two Apps of the same tool share a family resemblance
(materials, furniture, decor) but differ in wood tone, wall finish, layout
and sign. Different tools differ in at least three of: hue, floor pattern,
wall finish, furniture family.

| Tool | Family | Floors | Walls | Furniture | Nook and second setting | Sign |
|---|---|---|---|---|---|---|
| Claude Code | Workshop | parquet or planks, warm wood | brick, painted with a dado, or timber slats | wood desks, task chairs, pods or clusters | library or sofa; bookshelves and an armchair | banner or plaque |
| Claude app | Studio | pale oak planks or soft carpet | pastel paint or stripes | white tables, stools, laptops | sofa or huddle; a round table | banner |
| Codex | Industrial | polished concrete | brick or concrete panels | steel bench desks, mesh chairs, dual screens | huddle or workbench; lockers and a printer | neon or blade |
| OpenCode | Maker garage | rubber | ply slats or dark paint | standing desks, stools | workbench; ping-pong | neon |
| Gemini CLI | Campus | terrazzo or bold carpet | supergraphic stripes or acoustic panels | white desks, bright chairs, clusters | beanbags; ping-pong | pennant |
| Cursor | Lab | dark carpet tiles | acoustic panels | steel pods, mesh chairs | huddle | blade |

New tools get a family in their adapter, next to their icon and paths. The
renderer never branches per tool.

## Signs and wayfinding

- **One hierarchy, one position.** Every area has, at the same place on
  its back wall:
  - a flat, high-contrast plate with the tool's icon (a monogram in the
    prototype; the real build reads the installed app's own icon at run
    time) and the App name;
  - the App's accent colour on the plate's edge, so colour is never the
    only cue.

  The kit changes the plate's *style* (banner, neon, plaque, blade,
  pennant), never its place or reading order (ADA sign guide; UNSW sign
  standards; NIH signage manual).
- **Flags.** A pennant in the App colour stands at the aisle end of each
  area, so it reads from far away.
- **A landmark per floor.** The lift hub carries the floor numeral and a
  directory totem listing the floor's areas (SEGD's SAP case:
  lift banks as zone landmarks).
- **Labels when zoomed out.** Below area zoom, the overlay draws
  two-line name chips: App, then state. They are drawn in screen space, so
  they stay crisp at any zoom.

## The procedural layer

Seeded randomness, stable across launches:
- **Layout.** Desks follow the kit's layout and are placed by constraint:
  aisle clearance, chairs facing the screen, groups of desk, chair and
  screen together. Never a random scatter (Make It Home, Yu et al. 2011).
- **Props per desk.** Mugs, photos, sticky notes, books, headphones,
  figurines and tool-specific gear. Never identical across desks: Leesman
  finds personalisation matters to people.
- **Plants.** One to three per area from the kit's species, plus window
  boxes and hanging pots where the kit is green (Terrapin's 14 Patterns:
  visual connection with nature).
- **People.** Built from parts:
  - 9 hair styles, skin tones, 6 tops, glasses, beards and phones;
  - a badge in the App colour.

  Each person is seeded by App and desk, built once and cached.
- **Light.**
  - One global light direction.
  - Window light pools and lamp pools baked into the floor.
  - Wall ambient occlusion and desk shadows.
  - Each kit carries a warmth tint: warm pendants over lounges, cooler
    task light over desks.
- **Colour.**
  - Ramps are hue-shifted: shadows cooler, highlights warmer, about
    20° per step at most.
  - Outlines are selective.
  - People get the highest contrast in the scene (Slynyrd Pixelblog 1
    and 6; saint11).

## States

| State | What you see |
|---|---|
| Working | typing at the desk, screen lit |
| Asks you | stands and waves, with a bubble |
| Done | leans back, check bubble |
| Stuck or error | scratches their head, alert bubble |
| Limit | asleep at the desk |
| App overloaded | a small fire on one of its desks (people never burn) |
| Unused App | lights dimmed, dust, clutter, chairs pushed out, no people |
| This Mac overloaded | red alert: a red tint, a banner and beacons on every floor |

The overlay repeats each state as a word, so colour and pose are never the
only signals.

## Sources

Read for this note (fetched unless marked):
- Steelcase, Community-Based Design and Hybrid Needs a Home (search only).
- Gensler Atlanta neighbourhoods (search only).
- Herman Miller, Living Office (search only).
- Haworth, materials trends (search only).
- Terrapin Bright Green, 14 Patterns of Biophilic Design.
- Leesman, desk personalisation (search only).
- US Access Board, ADA Standards guide, chapter 7: Signs.
- NIH Interior Signage Manual (search only).
- SEGD, SAP America HQ case (search only).
- UNSW wayfinding signage standards (search only).
- Kate Compton, "So you want to build a generator" (secondary coverage).
- Game Developer, How Townscaper works.
- Maxim Gumin, WaveFunctionCollapse.
- Yu et al., Make It Home, SIGGRAPH 2011 (search only).
- Lospec palette list.
- Slynyrd Pixelblog 1, 6 and 42.
- saint11 tutorials.

The materials list (carpet tile, terrazzo, rubber, brick, acoustic panels)
is design judgement, not sourced.
