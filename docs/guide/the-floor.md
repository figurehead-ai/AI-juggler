# The Floor

The Floor shows all your AI work at a glance, as an office seen from
above. Each app is an area of the office, each session a person at a
desk, each subagent a helper beside them. Your accounts and your Mac
are in the lobby.

Everything on the Floor tells you something: where work is busy, waiting
on you, stuck, crashing or using too much, and which account is running
out. The furniture and plants are only there so each area is easy to
recognise. Click anything that means something and AI Juggler opens it.

The Floor is on macOS now. Windows gets it in 0.6 and Linux in 0.7; until
then its button says it isn't available on that system yet.

## Opening it

- The **Floor** button in the header, beside your menu.
- **Window › The Floor** in the menu bar.
- **The Floor** in the menu bar icon's menu.
- **⌘⇧M**. Press it again on the Floor to go back to the main window.

The Floor opens in its own window, on the floor with the app that most
needs you, or else on floor 1. Closing it stops its drawing and frees the
memory it used.

## Floors and areas

- Each floor has a **lobby** and up to 10 areas, in two rows either side
  of an aisle. The lobby has the lift with the floor number, the usage
  board for your accounts, your Mac's rack and the notice board.
- **An area grows with its app's sessions:** one cell for up to 6
  sessions, two for up to 12, four for up to 24. A busier app gets an
  annex, signed "App · annex".
- **Areas stay where they are.** Apps keep the order the Floor first saw
  them in. An area moves only when an app before it grows or shrinks, and
  then it moves forward. An area grows at once but shrinks only after a
  day, and a closed app's cells stay empty until the office is next laid
  out again.
- **Every app gets its own look**, chosen once and kept: Claude Code a
  workshop, the Claude app a studio, Codex an industrial hall, OpenCode a
  maker garage, other tools a plain office. Floors, desks, chairs and
  plants vary within that.
- Each pool has a meeting room on the floor where most of its members
  sit. Empty cells become the floor's kitchen and lounges.
- A session keeps its desk for as long as it is open.

## Finding an area

- **Its logo is printed on the floor:** the tool's mark (or the desktop
  app's own icon) with the initials of the app's account, in a ring of the
  app's colour.
- **Its sign** hangs over the way in: the app's full name, an edge in its
  colour and its account's battery. A flag in the app's colour stands
  beside it. Two apps of one tool on one account have the same logo; the
  colour and the sign tell them apart.
- **From far away** a chip floats over each area with its name and
  counts.

## What it shows

Colour is never the only sign: each state also has a pose, a symbol and a
word, in the tooltip and on the chip.

| What you see | It means | A click opens |
|---|---|---|
| Typing, desk lamp on | The session is working | The session |
| Leaning back with a mug | Open, not working | The session |
| Standing and waving, `!` and a spotlight; `!2` on the chip | Waiting on you: a question or a permission | The session, at the question |
| Scratching their head, `?`, a frozen screen; `?1` | Probably stuck | The session |
| Hands on head, `X`, smoke; `X1` | Crashed. The smoke stops after 10 minutes | The session, at the error |
| Stretching, a green tick | Done: worth a look. Back to idle after 15 minutes | The session |
| Asleep at the desk, `Z`; `OUT` | At its limit, or its account has run out | The account in Usage |
| Small helpers round a desk | Its subagents at work (6 drawn, then `+N`) | The session's subagents |
| A small fire on a desk; `HOT` | The app has used a large share of the processor for 2 minutes. Nobody burns | The app's processes |
| Dim, then dusty, then dark; `UNUSED` | The app hasn't been used for a day, a week, a month | The app |
| A battery on the sign, yellow then blinking; `12%` | The account is running low | The account in Usage |
| The rack's lights in the lobby | How busy your Mac is | This Mac |
| A slow red glow, a banner | Your Mac has been overloaded for 5 minutes. It never flashes | This Mac |
| A number on the whiteboard | The app's open tasks | Tasks, for that app |
| Notes on the notice board | Your notifications | Notifications |

Point at anything to see its name and state. Some things are just for
fun (the plants, the coffee machine, the ping-pong table, the office
cat): clicking them opens nothing.

## Moving around

- **Pan:** drag, or scroll with two fingers.
- **Turn and tilt:** right-drag, ⌥-drag, twist two fingers, or Q, E and
  the arrow keys.
- **Zoom:** pinch, ⌘-scroll, or + and −. Zoom out far enough and you see
  the whole building; zoom back in to the floor under the pointer.
- **H** goes back to the starting view. **0** shows the building, **1**
  to **9** pick a floor.
- Walls between you and the room fade away, with what hangs on them.

From the keyboard alone: **Tab** and **Shift-Tab** move from area to
area (the view follows), **N** jumps to the next session waiting on you,
and **Enter** opens what's selected.

The window's title always says what's on screen, for example "The Floor
— Floor 2: 6 Apps, 3 waiting on you".

## Settings

Under **Settings › The Floor**:

- **Open the Floor when AI Juggler starts** and **Keep it on top**, both
  off at first.
- **Apps per floor**: 10, or anything from 4 to 16.
- **Motion**: follow the system's Reduce motion setting, Full, Calm
  (slower, nobody walks about) or Off.
- **Show unused apps' decay**: on at first.
- **Reset the view**, and, under More, the apps hidden from the Floor.
- **Re-create the office…** picks a new look and layout for every area
  and lays the office out again, in the same order. It asks first, and
  changes nothing outside the Floor.

## What it costs

The Floor draws only when something changes or moves. Open and still, it
uses next to nothing; covered, minimised or closed, it doesn't draw at
all. It keeps the logos it draws in AI Juggler's local cache, which you
can see and clear under Settings › Advanced › Local cache.

## Without the Floor window

The main window's **Floor** list shows the same things in words, grouped
by floor and app: each session's state, its subagents, tasks and
project, and **Show on the Floor**. It works with VoiceOver and the
keyboard like the rest of AI Juggler.
