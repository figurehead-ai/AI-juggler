# AJ-E812 and AJ-E814 — idle CPU, 0.4.6

2026-11-16, reviewer claude-opus. Machine: the reference Mac (Apple M5
Ultra, 30 cores, macOS 27), shared with other sessions and the
maintainer's own work while it was read. Follows
[`idle-0.4.5.md`](idle-0.4.5.md) (AJ-E805), whose round 5 left two
things open: the web content's and GPU process's spikes (3–8 % of a core
for part of a minute, in both of its builds, under load) and a window
shown but fully covered.

## Method

No `sample`, `lldb`, `dtrace`, debugger or profiler was attached to any
process (round 5 used `sample`; this round doesn't). What was read:

- **CPU, by pid only.** `ps -o time=` of the lab build's `juggler-app` and
  of the WebKit processes it is responsible for (web content, GPU,
  networking; libsystem's responsible-pid call), every second, with the
  1-minute load average beside each line (`scripts/qa/idle_cpu.py
  watch`). A reading is the three together, % of one core, over 60 s.
- **What the page does, from inside it.** A measurement-only build (never
  committed; an export of 078fe682f with the probe below, the UI built
  unminified) whose page logs each second: React commits and the
  components that rendered (a stub React DevTools hook, as
  `scripts/qa/idle_renders.mjs`), rAF, timer, interval and
  ResizeObserver callbacks by caller, running CSS animations
  (`document.getAnimations()`) and their start and end events, DOM
  mutations by the nearest named element, the backend events the page
  heard and the commands it called, the visibility state; ten seconds at
  a time to a file in the lab through a probe-only command.
- **What happened to the window from outside,** from the app's own system
  log (`log show` by its pid; `idle_cpu.py events`): display
  reconfigurations, the app made active, the window occluded or shown
  again, a close pressed from outside, accessibility clusters. The
  window server's uncovered share from `CGWindowListCopyWindowInfo`
  (bounds and order only; `scripts/qa/window_cover.swift share`).
- **Labs.** `/tmp/aij-lab-idle46`: `scripts/qa/lab_scenario.sh setup PD3
  --import` (the replica of a heavy Mac plus PD3: 12 Accounts, 4
  Projects, 52 ended sessions; 334 transcripts). *Working*: `replica.py
  live --claude 7 --codex 3 --no-desktop` with a fixed lifetime, its
  transcripts fresh at each start (the one reader per transcript makes
  their length not matter, round 5).
- **Launch.** `open -g -n --env HOME=<lab>`: behind the front app, never
  activated, then by pid only; ended by TERM on the pid.
- **The bar runs** (`scripts/qa/idle_run.sh`, one launch each): 75 s after
  launch, 60 s shown; then 60 s covered, 10 s after a lab-owned panel
  was ordered just above the window (`window_cover.swift cover`: an
  opaque, non-activating panel of an accessory app that can't become
  key, the window's exact size and place, ordered by the window's
  number; nothing else is activated or ordered); then 60 s hidden, 15 s
  after the lab app was hidden by pid. The panel first let clicks
  through: a click on it in launch after-5 reached the lab window under
  it, and the lab app made itself front (its own log: "App nap disabled
  for page due to user activity", then the window server's deferred
  ordering from the app's own connection). From launch 6 on it takes
  clicks itself and does nothing with them, and says on it "AI Juggler
  test: measuring, closes by itself in a minute". The before and after
  builds alternate, one launch at a time, under one set of 10 working
  stand-ins.
- **The drop rule** (round 4's, decided from the system log, never from
  the number), widened: a reading is dropped when its pid logged a
  display reconfiguration, the app made active, the window occluded or
  shown again, a close pressed from outside, or more than 3
  accessibility clusters a minute (a steady outside poll, every ~32 s,
  is in every reading). Every drop is listed with its reason.

## What the spikes are

### Reproduced, and the page did nothing new during them

Phase A (11:08–11:35), the probe build shown with 10 working:

| Run | Load (1 min) | 60 s readings, all three (%) | Kept | Over 2 % | Over 3 % |
|---|---|---|---|---|---|
| 1, 13 min | 9–27 | 1.32 1.67 1.27 1.82 1.87 1.45 ~~2.07~~ ~~1.80~~ 1.40 2.56 1.89 1.96 1.31 | 11 | 1 | 0 |
| 2, 6 min | 19–95 (other builds) | 2.71 3.65 3.09 1.49 ~~2.55~~ 1.94 | 5 | 3 | 2 |

Struck out: dropped by the rule (run 1: three display reconfigurations
at 11:15:35–38 and the app made active for 1.6 s at 11:16:52 by the
window server's front-process change, not by the lab; run 2: three
display reconfigurations). Each cost 4.1–4.7 % over its 10 s.

In run 2 the Mac's own load climbed to 85–95 and the readings rose to
3.1–3.7 % with nothing new on the page: per 10 s the web content read up
to 2.3 % and the GPU process 0.5–2.2 % (0.0–0.3 at low load), while the
page did what it does every minute — 4–11 React commits, 0–5 seconds
with a DOM write, no animation running and no rAF callback at any time
in either run (15 + 6 min watched). With no DOM write in a 10 s window
the GPU process stayed at 0.1–0.3 % even at load 80.

### The page's work, exactly counted

The probe's render counts first counted a subtree React skipped on every
commit (its fibers keep the PerformedWork flag of their last render);
`idle_renders.mjs` had the same flaw. Both count a component now only
when it ran in that commit (b11d39e1b; checked on a page with known
renders: 20 leaves that rendered once read 220 before, 20 after).

Shown, 10 working, exact (the 6-min count run, 11:41; load 6–18):

| What | Per minute |
|---|---|
| React commits | 19.5 |
| Components rendered | 1,215 |
| Seconds with a DOM write (paints) | 4.5 |
| Events the page heard | conversations 15.4, processes://counts 11.0, This Mac 12.0, notifications 1.9 |
| Commands it called | relation counts 10.3, remote Macs 10.6, the shell's lists 1.7 each, link 3.1 |
| Animations, rAF | none |

Renders by what started them: the lists read after a conversation
changed 40 % (67 % in a busier minute), the shell's 30 s clock and its
other state 34 %, the 2-minute limits and remote reads 21 %, This Mac's
5 s reading 4 % (5 components a reading since AJ-E813). One shell
render draws about 300 components; most show what they showed before:
Home's App tiles' icons, profile badges and kind marks (15, 15 and 12 a
render), the runway's rows, the rail's inactive Apps.

`processes://counts` reached the page 11–15 times a minute and drew
nothing: its only reader, the top bar, shows `to_clean`, and the event
was sent when any count moved (the stand-ins' processes come and go).

### Paint effects make no difference (the A/B)

The probe build alternated 60 s phases, normal and "flat" (no
`backdrop-filter`, `filter` or `box-shadow` anywhere), under a
controlled load of 20 bounded `yes` (load 25–29, 10 working, 11:50–12:01;
`pgrep -x yes` 0 after):

| Phase | Readings (all three, %) | app | web | GPU |
|---|---|---|---|---|
| normal (4 × ~57 s) | 1.53 2.13 2.06 1.38 (mean 1.78) | 1.03 | 0.52 | 0.23 |
| flat (4 × ~57 s) | 1.53 2.04 2.04 0.89 (mean 1.63) | 1.03 | 0.48 | 0.12 |

The spread follows the page's own work in that minute (14–41 commits,
2–14 seconds with a DOM write), not the arm: the normal phases happened
to hold 37 seconds with a DOM write and the flat ones 26, which is the
GPU process's 0.11 between them (its cost per second with a write is the
same in both, 0.02–0.03 %). A load of ~30 from `yes` amplified
nothing either: the GPU process read as at low load. The
amplification of phase A came only under real contention (builds and
the maintainer's work, load 85–95, memory-heavy and wider than the 30
cores); a high-load A/B was not run, as the Mac was in use (load 83 from
the maintainer's own work at the end of the A/B).

### Outside events, and the window moved

In phase A's 20 minutes shown the window was disturbed from outside
three ways, each named in the app's own log and none by the lab:

- **Display reconfigurations** (11:15:35–38, 11:31:21–24): the external
  display at the left hot-plugged out and back (WindowServer: `Display 4
  hot plug 0`). The window on it moved to the main display (0 %
  uncovered there, under other windows) and back. 4–5 % for 10 s.
- **The app made active** (11:16:52.9, 1.6 s): the window server made the
  lab build the front process and gave the front back to the
  maintainer's app (a ⌘-Tab or a click). 4.1 % for 10 s.
- **Moved off screen:** at 11:22:23 the window was at x = 12 instead of
  1727, and at 11:22:40 the window server set it hidden (`window
  occluded 1`; WebKit's page went hidden); at 11:33:40 again, on the
  left display (`windowDidChangeOcclusionState`). Those runs' remaining
  shown minutes were not readings.

## The cause

**There is no spike source.** Nothing in the page starts, loops or draws
on its own while nothing changes: no animation, no rAF, no timer of its
own beyond the 30 s clock and the windows' reads. The 3–8 % readings are
the page's routine work — the events it hears, the commits they lead
to, and the few paints a minute — made 5–10 times dearer for the web
content and GPU process by real machine contention (load 85–95), plus
outside events that make WebKit draw the whole window (a display
reconfiguration, the app made active). So the fix is less routine work:

1. **An event that draws nothing:** `processes://counts`, 11–15 a minute.
2. **Parts that draw what they showed before:** on every shell render
   (each list read, the 30 s clock) each App tile's icon, profile badge
   and kind mark, about 70 of the ~300 components a render.

## The fixes

| Commit | What | Guard |
|---|---|---|
| c003c0c75 | `processes://counts` is told only when the top bar's count (`to_clean`) moves; a reader of any other count listens to the graph's event (`onProcessCounts`' doc says so) | `tests/processes.rs::counts_the_top_bar_doesnt_show_tell_the_window_nothing`: on the old rule it fails at its first tick |
| be1e423f6 | `AppIcon`, `AppIconTile`, `ProfileBadge` and `AppKindBadge` draw again only when their own props change (`memo`; the icon's read and the marks' store still draw them) | `components/leaf-draws.test.tsx`: without `memo`, "the icon, its props unchanged: expected 14 to be 4" |
| b11d39e1b | `idle_renders.mjs` counts a component only when it ran in that commit | the known-renders check in its message |
| this record's commit | The measurement tools: `scripts/qa/idle_cpu.py` (CPU by pid, windows, the outside events), `window_cover.swift` (uncovered share, the labelled cover panel, hide by pid), `idle_run.sh` (one launch: shown, covered, hidden) | every bar reading below |

What they take away, from the exact counts before them (10 working,
shown): 11–15 of the ~45 events a minute the page heard and drew
nothing for, and up to ~290 of its ~1,200 renders a minute (each App
tile's and rail row's icon, its tile, badge, kind mark and glyph,
re-drawn with the same props on each shell render). No interval was
raised and no reading was read less often.

## Results

The Dev edition's release builds, each from an export: **before**,
078fe682f (AJ-E813 in); **after**, the same with c003c0c75 and
be1e423f6 alone. Seven launches each, alternating before and after,
12:40–15:02 with a pause from 12:46 to 13:54 for the maintainer's
sign-in sitting, 10 working throughout (`idle_run.sh`; readings by pid
every second, the 1-minute load logged with each). The hidden readings
were taken from the same files with the last sample before the minute's
end where the watcher had already stopped (it was stopped a moment too
early; the committed script waits 2 s for it).

**Shown, 10 working, 60 s readings, all three processes (%):**

| Pair | Before | load | After | load |
|---|---|---|---|---|
| 1 | 1.42 | 5 | 1.45 | 20 |
| 2 | **7.10** (web 2.49, GPU 2.54) | **73** | 2.33 | 26 |
| 3 | 2.20 | 22 | 1.71 | 12 |
| 4 | 2.03 | 21 | 1.64 | 9 |
| 5 | 1.55 | 14 | 1.78 | 14 |
| 6 | 1.48 | 7 | 1.58 | 7 |
| 7 | ~~2.88~~ (four display reconfigurations) | 8 | 1.31 | 6 |

Pair 2 ran while a virtual machine and other work of the maintainer's
held the Mac at load 67–78 (neither session's): the before build's
reading is the spike again, live (web content 2.49 %, GPU 2.54 %, no
outside event in its log). It stays out of the gate, as the coordinator
ruled: the gate takes pairs at low load only.

| Build | Low-load readings (pairs 1, 3–7) | **Median** | Over 2 % | Over 3 % |
|---|---|---|---|---|
| before | 1.42 2.20 2.03 1.55 1.48 | **1.55** | 2 of 5 | 0 |
| after | 1.45 1.71 1.64 1.78 1.58 1.31 | **1.61** | 0 of 6 | 0 |

**The bar is met with the fixes in:** five kept 60 s readings and more,
shown with ten working, none over 3 %, the median 1.61 % (the first
five: 1.64). The share of 60 s readings over 2 % went from 2 of 5 to 0
of 6 at low load (with pair 2: 3 of 6, one over 3 %, to 1 of 7, none
over 3 %). At low load the fixes' own CPU is inside the readings'
spread: pair by pair the differences run −0.23 to +0.49, the larger
ones where the before reading had the higher load (pairs 3 and 4, load
21–22 against 9–12). What they remove is counted (above); what the
readings show is that the shown window's work no longer reaches 2 % at
the loads it reached it before. The 0.5 session's own reading of its
build without them, the same morning at load 5–10: 1.48 % shown (60 s
median), 2.29 % (one 180 s reading), hidden 0.82 and 0.74 %.

**Hidden** (the lab app hidden by pid, 60 s): before 0.75 0.85 0.73
0.68 0.83 0.85 0.91, median **0.83**; after 0.75 1.70 0.72 0.70 0.80
0.99 0.91, median **0.80** (the 1.70 at load 68). The web content
0.02–0.08 % and the GPU process under 0.05 % in every low-load one.

Dropped, by the rule (the app's own log): before-1 covered (three
display reconfigurations, then a close pressed from outside at 12:43:59:
`performClick:`, `windowShouldClose: prevented close`, as round 4 saw),
before-7 shown (four display reconfigurations) and covered (the window
shown again and the app made active), after-5 covered (a click on the
cover panel; see Method). A launch of the after build was voided when
the maintainer's sign-in sitting needed the Dev edition (12:45); it
isn't in the tables.

## AJ-E813's effect

AJ-E813 (1d205929d) is in both builds. A This Mac reading now draws 5
components (the provider, its value, the header's status line, its
icon, the notices' openers); before it, each reading also drew Home on
its own, about 125 components (a Home render of its own in the counts),
12 times a minute: about 1,500 renders a minute less, more than half of
what the page drew before it (about 2,700 a minute with ten working).
Its CPU effect alone wasn't read: no build without it was run in the
window. The bar above is met with it in, at 1.61 % (round 5's after
build, before AJ-E813 and these fixes: 1.75 %, at load 19–40).

## AJ-E814: covered

The covered state is a lab-owned panel just above the window, its exact
size and place: the window server's list put the panel directly above
the lab window and the window **0.0 % uncovered** in every covered
reading kept, and WebKit logged the page hidden at the cover (`view
visibility state changed 1 -> 0`, after a few flips in the first 0.4 s
while the panel was ordered, then hidden for the whole minute). A fully
covered window is a hidden page to WebKit.

| After build, pairs 1, 3, 4, 6, 7 | Readings (%) | **Median** |
|---|---|---|
| Shown | 1.45 1.71 1.64 1.58 1.31 | **1.58** |
| Covered | 0.98 0.74 0.93 1.01 0.98 | **0.98** |
| Hidden | 0.75 0.72 0.70 0.99 0.91 | **0.75** |

(The before build, covered, low load: 0.96 0.81 1.07 0.93, median
0.95; one more kept at load 78: 2.43.)

**The covered median is under the shown median** (0.98 against 1.58):
covered, the web content falls from 0.36–0.62 % to 0.05–0.16 % and the
GPU process from 0.05–0.18 % to 0.00–0.05 %, as hidden. **The gap to
hidden** (0.98 against 0.75) is the app process (0.70–0.90 % covered,
0.62–0.91 % hidden) and the web content's last 0.05–0.10 %: a covered
window is still on screen and in the window list, so AppKit and WebKit's
UI side keep serving it (WebKit logged no activity state change in the
covered minute after the first 0.4 s), and each covered reading came
75 s earlier in its launch than the hidden one, while the app's own
start-up work still tails off. Not traced further: the gap is within
the app's own reading-to-reading spread (0.62–0.91 % hidden alone).

## Open

- **The shell's clock and lists draw the whole app.** One shell render
  draws ~300 components; the 30 s clock and every list read do one.
  Components that show a time subscribing to the clock themselves (the
  AJ-E813 pattern) would remove most of the rest; a refactor of the
  shell's `now`, not done here.
- **This Mac's reading carries the whole page** every 5 s (trees, other
  apps, alerts) while the window reads only its summary and heavy notes
  outside This Mac's page.
- **Real contention multiplies the window's cost 5–10×** (pair 2,
  phase A run 2): nothing of ours starts it, and the less the shown
  window does, the less it multiplies. A high-load A/B under one's own
  control (load 80–90) wasn't run: the Mac was in use.
- **Outside events on the shared Mac** (the left display hot-plugging,
  the lab window moved, made front by a click, closed through the
  accessibility API) drop readings in every round; the rule names each.
