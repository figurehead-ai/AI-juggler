# The Floor — rules and mechanics

> Status 2026-11-03, round 2. The maintainer liked the first Floor and asked for:
> - more model detail;
> - packed areas, not one column per App;
> - a floor limit;
> - stable reflow and a look that sticks;
> - desks not all in rows;
> - fun hover and click touches;
> - every visual tied to something useful: where it is busy, stuck, crashing or overloaded, and
>   which Account is running out.
>
> They also asked whether real users can rotate and angle the view. **Yes.** This round the Floor
> is real 3D (low-poly models drawn as pixel art). You can:
> - turn it all the way round;
> - tilt it between about 18° and 79°;
> - zoom from one desk to the whole building;
> - press H to return to the default angle.
>
> The first prototype was flat art painted at one fixed angle, which is why it could not turn.
>
> The prototype for this note is `spikes/floor-office/` on the branch `proto/floor-office-2`. The look's design is in
> [`style-kits.md`](style-kits.md).

## 1. The one rule

**Everything you see is a signal or a way to find one.** Decoration exists to make areas
recognisable (wayfinding) or the office pleasant to look at, and it never competes with a signal.
Signals get the strongest contrast, motion and light. Furniture and decor stay calmer.

## 2. Signals

Each signal shows in three places: up close, from far away (the area chips and building view), and
in the main window. Colour is never the only cue: every state also has a pose, an icon and a word.

| Signal (source) | Up close | From far | Limits | A click opens |
|---|---|---|---|---|
| **Working** (session running) | Typing at the desk, code scrolling on the screen, desk lamp on | — (the normal state, kept quiet) | Animation only while the area is in view | The session |
| **Idle** (session open, not working) | Leaning back with a mug, still screen | — | — | The session |
| **Waiting on you** (session asks for input or a permission) | Stands and waves, a yellow `!` balloon, a spotlight cone on the desk | `!N` on the area chip; the spotlight and balloon still show at floor zoom | One balloon per desk; the chip counts the rest | The session, at the question |
| **Stuck** (running, no progress for N minutes) | Scratches their head, `?` balloon, frozen screen, amber lamp pulsing | `?N` on the area chip | N is a setting (default 10 minutes) | The session |
| **Crashed / error** | Hands on head, red `X` balloon, red error screen, smoke over the desk, red light | `XN` on the area chip | Smoke stops after 10 minutes unless it happens again | The session, at the error |
| **Done** | Stretches, green tick, green screen | — | Shown for 15 minutes, then idle | The session |
| **Subagents** (a session's helpers) | Helpers in the App's colour stand round the desk with clipboards, looking at the screen | — | At most 6 drawn; above that a `+N` tag | The session's subagent list |
| **App overloaded** (the App's CPU share over its threshold for a while) | A small fire on its busiest desk, with smoke and a flickering orange light (people never burn) | `HOT` on the area chip | One fire per App | The App, at its processes |
| **App unused** (closed or idle) | Under a day: lights dimmed, chairs pushed out. Over a week: dust, boxes, wilted plants. Over a month: lights off. | `UNUSED` on the chip | Desks are kept for a day after a session ends | The App |
| **Account running low** (usage left) | Every area's sign has a 4-segment battery for its Account: green, then yellow under 40%, then amber and blinking under 20% | `12%` on the chip; the lobby's **usage board** shows every Account's gauge | — | The Account in Usage |
| **Account out** | Everyone in its Apps sleeps at their desk (`Z` balloons), screens off, red battery | `OUT` on the chip | — | The Account in Usage |
| **This Mac's load** | The lobby's server rack: lights rise with the load, green to amber to red | — | — | This Mac |
| **This Mac overloaded** (over its threshold for minutes) | Red alert: a red wash, beacons turning at every lift and every sign, a banner | The banner shows from any view | Ends with hysteresis (under the threshold for a minute) | This Mac |

## 3. Layout

### Floors and cells

- **A floor is a lobby plus a grid of cells.** The lobby holds the lift with the floor number, the
  directory, the usage board, This Mac's rack and a bench. The grid has two rows (back and front)
  with an aisle between them, and as many columns as half the floor's App limit.
- **At most 10 Apps a floor by default**, a user setting. With 10 the grid is 5 × 2. A column holds
  at most two areas, one above the aisle and one below.
- **An App's size follows its sessions:**

  | Size | Sessions | Cells | Desks |
  |---|---|---|---|
  | S | up to 6 | 1 | 6 |
  | M | up to 12 | 2 side by side | 12 |
  | L | up to 24 | a 2 × 2 block across the aisle; the aisle runs through the team | 24 |

  Beyond 24, an annex of the same App opens next in order, signed "App · annex".
- **Free cells** become the floor's kitchen and lounge (the first free cell) or a lounge with a
  "free space" sign.

### Placement order and reflow

- **The order is fixed: the order the Floor first saw each App.** Activity never reorders it.
- **Apps are placed first-fit:**
  - columns left to right, the back row before the front;
  - on the first floor with room;
  - a new floor opens when none has room.
- **Stability rule.** An area moves only when an App before it in the order changes size. It then
  moves forward in the same reading order. Areas before the changed App never move.
- **Grow at once, shrink slowly.** An App grows to a bigger size as soon as it needs to. It shrinks
  only after a day below the smaller size, so areas don't flicker.
- **Closing an App** leaves its cells empty (a lounge) until the next reflow or Re-create. Nothing
  moves just because something went away.

### Choices that stick

The first time the Floor sees an App it chooses a **layout type** and a **seed**, and saves both
(`floor-state.json` in the prototype; the app's local store in the build). Everything about the
area's look follows from them and the tool's family:
- floor, wall, desk model, chairs;
- plants and pot, divider, nook, sign, rug, lights.

Restarts, new versions of the kits and reflows keep it. **Re-create the office** (a setting, with a
confirmation) picks again for every App.

### Desks inside an area

- **Layout types per family** (see `style-kits.md`):
  - pods of four;
  - benches;
  - pinwheels of three at 120°;
  - scattered freestanding desks;
  - long shared tables;
  - diagonal rows at 45°;
  - radial desks round a planter.
- **Not all in rows.** Groups take a seeded turn: none in strict families (Industrial, Lab), up to
  about 7–17° in calm ones, and loose angles and positions in Scatter.
- **A session keeps its desk for life.** Desk *i* belongs to session *i* while the area keeps its
  size. Two spare desks stand ready, so a new session sits down without anyone moving.
- **The team sits by the way in.** Groups fill from the aisle side first. Each area has a nook
  (sofa, library, huddle, workbench, beanbags, ping-pong or arcade) in a corner away from the aisle,
  and 1–3 plants.

### Finding your way

- **Every area has the same sign in the same place:** a plate hung over the way in, readable from
  both sides. It shows the tool's icon (a monogram in the prototype), the App name, an edge in the
  App's colour and the Account battery.
- **The tool's logo and the Account's initials are printed into the area's floor** at 35%
  opacity (the maintainer asked for 30–40%, D-67): "logo like: Codex, Claude Code along with
  initials of the account".
  - It is large, centred in the area, part of the floor's material, and readable from the
    default angle.
  - The tool's mark is drawn as AI Juggler draws it elsewhere (`ToolMark`, or the tool's own icon
    read at run time). The initials come from the Account's name (`accountName`).
  - Fallbacks: no known Account gives the App's own initials; no tool mark gives the tool's name in
    letters. Nothing is bundled.
  - Two Apps of one tool on one Account carry the same logo. They are told apart by the App's
    colour, which rings the logo, and by the App's full name on the area's sign.
- A **flag** in the App's colour stands at the corner of the way in.
- The **lift** is each floor's landmark: the floor number above its doors, and a **directory** of
  the floor's areas with their colours.
- From far away, **area chips** float over every area: name plus counts (`!2 ?1 X1 12% HOT`).

## 4. Models

Low-poly, but each thing is shaped like itself, never a plain box:

- **Desks**, 6 kinds:
  - four-leg with an apron;
  - trestle with A-frame legs;
  - pedestal with drawers;
  - standing desk on T-legs;
  - L-shaped;
  - round on a pedestal.

  Shared tables have a cable spine.
- **Chairs**, 5 kinds:
  - task chair: five-star base, casters, gas lift, seat, curved back, arms;
  - mesh chair with a headrest;
  - stool with a foot ring;
  - saddle stool;
  - exercise ball.
- **Plants**, 8 kinds:
  - monstera: split leaves on stems;
  - fern: arching fronds;
  - snake plant: upright blades;
  - cactus: arms and a flower;
  - fig tree: trunk and canopy;
  - palm: segmented trunk and fronds;
  - herbs;
  - succulent rosette.

  They sit in 5 kinds of pot: terracotta, ceramic, planter box, basket, concrete.
- **Screens:**
  - single, dual angled, ultrawide, laptop;
  - keyboards and mice;
  - the screen shows the state.
- **Desk props**, a different set per desk:
  - mugs, a succulent, books, a framed photo;
  - a notebook with sticky notes, a bottle, headphones;
  - lamps with a real light.
- **Furniture:**
  - sofas, armchairs, bookcases, a huddle table and whiteboard;
  - a workbench with pegboard and a bike;
  - beanbags, ping-pong, an arcade cabinet, lockers;
  - a kitchen with a fridge, a coffee machine.
- **People** are built from parts: shoes, legs, hips, torso, arms, hands, neck, head, eyes, mouth.
  Looks vary:
  - 9 hair styles, skin tones;
  - 4 top styles (hoodie, collar, stripe);
  - glasses, beards;
  - height.

  Every person wears a badge in the App's colour. Poses: typing, idle with a mug, waving,
  scratching, hands on head, stretching, asleep, helping, walking.

## 5. Interaction

- **Hover:** the thing under the mouse gets a light outline and a tooltip (name and state). A person
  looks up at you.
- **Click to go:**
  - an area opens its App;
  - a desk or a person opens the session;
  - a helper opens the subagents;
  - a usage gauge opens the Account in Usage;
  - the rack opens This Mac;
  - the lift or directory goes to that floor.
- **Click for fun** (these never navigate):
  - plants rustle;
  - ping-pong plays a rally;
  - the coffee machine steams;
  - beanbags squash;
  - the arcade lights up;
  - the cat on floor 1 sits and purrs.
- **The camera:**

  | Do this | To |
  |---|---|
  | Drag, or scroll with two fingers | Pan |
  | Right-drag, ⌥-drag, a two-finger twist, Q / E or the arrow keys | Turn and tilt (tilt about 18°–79°) |
  | Pinch, ⌘-scroll or + / − | Zoom (towards the cursor) |
  | Zoom far out | Shows the building |
  | Zoom back in | Lands on the floor you are over |
  | H, or Reset the view in the menu | Return to the default angle |
  | 0, 1, 2, or the chips | Building, floor 1, floor 2 |

  Walls between you and the room vanish as you turn (the cutaway). The things hung on them vanish
  too.

## 6. Budgets and draw policy

- **Draw only on a change:**
  - input or a camera move;
  - an animation tick, only while something moving is in view and the window can be seen;
  - nothing while covered or minimised.
- Only the floor in view is drawn; the building view draws every floor.
- **Caps:**
  - 6 helpers drawn per desk;
  - the 96 nearest lights;
  - 60,000 moving parts per frame;
  - one fire per App.
- **The pixel look.** The scene renders at 1/3 resolution on Retina (1/2 on 1× screens) and is
  scaled up with hard edges, so it reads as pixel art. Outlines come from the pick buffer, which also
  gives exact picking.

## 7. Accessibility

The canvas cannot be read by a screen reader, so the main window keeps an accessible list of the
same areas, desks and states (E35's list view). Every state on the Floor also has a word: in
tooltips, chips and the list.

## 8. What the prototype shows, and what is only specified

**Shown in `spikes/floor-office/`.** Sample data with generic names:
- every signal in the table, except stuck and crashed timers (fixed states);
- the packed cells;
- first-fit order and the reflow demo (G, or the tour's last two frames);
- choices saved to `floor-state.json`, and Re-create;
- every layout type, with its fallback (a crowded area gives up its nook, then uses pods, before
  anyone goes without a desk);
- the models;
- hover, the clicks, the fun props and the camera.

**Specified only, for the build:**
- annexes beyond 24 sessions;
- the one-day shrink delay;
- vacated cells waiting for the next reflow;
- the timers for stuck, done and smoke;
- the Floor's App limit as a real setting (the prototype has 10);
- tool icons read at run time;
- Windows and Linux input;
- the accessible list in the main window.
