# AJ-E805 R3: idle CPU, 0.5 before the render-subscription fix

2026-11-16, Sonnet measuring run. **This is the "before" reading**: the
current 0.5 build, before the render-subscription fix. A matching "after"
reading follows once that fix is in.

Method: exactly `idle-0.4.5.md` (round 4/5: the Dev edition's release build,
the grown lab, the window shown, then hidden, 10 sessions working, 60 s
readings with the median of five, and 180 s readings; the processes summed
are `juggler-app`, its WebKit web content process and its WebKit GPU
process, by pid). `release-idle.txt` (the 0.4.3 candidate on the synthetic
fixture) is for contrast only.

## Setup

- **Build.** The Dev edition's release build at commit `627bd53b8`
  (release/0.5), built earlier in the shared target; copied with `ditto` to
  a scratch folder, signed ad hoc as a whole (`codesign --force --deep
  --sign -`) and checked with `codesign --verify --strict`. Started by its
  binary only (never `open`), with the lab's `HOME`, by pid. Nothing named
  "AI Juggler Dev" was running before; the installed app was not touched.
- **Lab.** `scripts/qa/lab_scenario.sh setup PD3 --import` on a fresh
  throwaway home (the replica of a heavy Mac, 7 Claude and 2 Codex
  Accounts, 340 conversations, brought in as Accounts plus PD3's own
  state), then **grown**: each of the 336 Claude transcripts cloned 15 times
  (`cp -c`): 5,376 in all (as 0.4.5's 4,832 for 302).
- **Sessions.** Working: `replica.py live --claude 7 --codex 3
  --no-desktop` (seven Claude and three Codex stand-ins, a turn every
  about 4 s). Six of the seven Claude stand-ins write a growing transcript:
  before the run those six were set to a size of about 1 MB (their own
  lines repeated, cut at a whole line) and the stand-ins restarted on the
  same files: **1,000,063 to 1,000,407 bytes each at the start** (1,000,063;
  1,000,090; 1,000,135; 1,000,275; 1,000,286; 1,000,407); they grew to
  1,306,259 to 1,311,581 bytes by the end (about 16 KB a minute each). The
  three Codex stand-ins start a new rollout each time (a few KB).
- **Readings.** `ps -o time=` of the three pids (cumulative CPU time) at the
  start and end of each reading, % of one core; `sysctl -n vm.loadavg` at
  the start and the end of each reading. The window first shown 150 s after
  launch (launched 12:08:24, first reading 12:10:55), then hidden through
  System Events by pid (`visible` false), 15 s, then the same readings.
  The web content and GPU processes were found by their start time (both
  within a second of the app's) and the shared responsible pid; the
  networking process and the app's `--aij-watch` helper are not in the sum
  (the helper read 0.02 to 0.05 %, shown in its column).

## Readings

% of one core; "All" = `juggler-app` + web content + GPU. Load is the
1-minute figure at the start of each reading (5- and 15-minute figures in
the raw log).

| State | Reading | `juggler-app` | Web content | GPU | **All** | Load (1 min) start, end |
|---|---|---|---|---|---|---|
| Shown | 60 s #1 | 0.783 | 0.700 | 0.050 | **1.533** | 5.62, 5.16 |
| | 60 s #2 | 0.717 | 0.483 | 0.033 | **1.233** | 5.16, 5.71 |
| | 60 s #3 | 0.900 | 0.533 | 0.117 | **1.550** | 5.71, 6.68 |
| | 60 s #4 | 0.783 | 0.383 | 0.033 | **1.200** | 6.68, 7.07 |
| | 60 s #5 | 0.883 | 0.517 | 0.083 | **1.483** | 7.07, 7.35 |
| | **Median of 5** | 0.783 | 0.517 | 0.050 | **1.483** | |
| | 180 s | 1.378 | 0.672 | 0.244 | **2.294** | 7.35, 6.78 |
| Hidden | 60 s #1 | 0.817 | 0.033 | 0.017 | **0.867** | 6.48, 7.26 |
| | 60 s #2 | 0.767 | 0.017 | 0.033 | **0.817** | 7.26, 10.34 |
| | 60 s #3 | 0.667 | 0.000 | 0.017 | **0.683** | 10.34, 7.95 |
| | 60 s #4 | 0.633 | 0.050 | 0.033 | **0.717** | 7.95, 6.71 |
| | 60 s #5 | 0.800 | 0.000 | 0.017 | **0.817** | 6.71, 6.27 |
| | **Median of 5** | 0.767 | 0.017 | 0.017 | **0.817** | |
| | 180 s | 0.717 | 0.006 | 0.022 | **0.744** | 6.27, 7.27 |

No reading was taken at a 1-minute load above 20 (highest 10.34). The
15-minute figure was above 20 for the first shown readings (27.24 falling
to 21.15 by the sixth, 13 to 18 for the hidden ones): the Mac had been
heavily loaded just before.

## Comparison

| | 0.4.5 (`idle-0.4.5.md`, after build, 60 s median of 5) | 0.5 before (this file) | Budget |
|---|---|---|---|
| Shown, 10 working | 1.75 % | **1.48 %** | under 2 % |
| Hidden after a show, 10 working | 0.78 % | **0.82 %** | under 2 % |

**Verdict: under the 2 % budget in both states** on 60 s medians (1.48 shown,
0.82 hidden). The shown 180 s reading is **2.29 %, over 2 %**: one
reading, its `juggler-app` 1.38 and GPU 0.24 above every 60 s reading's
(0.72 to 0.90 and 0.03 to 0.12) for a reason not found; 0.4.5's own 180 s
readings were 1.32 to 2.19 and its 60 s medians were the bar. The hidden
180 s reading is 0.74 %.

Not measured here (the "after" reading's job): the render-subscription fix
itself.

## Conditions and caveats

- **The window was in front.** Starting the binary directly activated it
  (System Events named its pid the frontmost process at every shown
  reading), unlike 0.4.5's `open -g`. It was on screen and drawn; the
  uncovered share was not read from the window server this time.
- **Other heavy work on the Mac,** logged at the start of each reading
  (`ps -Aro`): a virtual machine (`com.apple.Virtualization.VirtualMachine`)
  at 104 to 120 % throughout, `fseventsd` at 99 to 177 %, `WindowServer`
  at 17 to 139 %, intermittently `rustc`, `node`, `mediaanalysisd`, `mds_stores`
  and a VPN's helper. None are the app's processes.
- **`fseventsd`'s load** is the Mac's whole file-event volume (the lab's 5,376
  transcripts and the stand-ins' writes are part of it); the app's readers
  work from those events.
- The spread of the shown 180 s reading (2.29) against the five 60 s
  readings (1.20 to 1.55) was not investigated.

## Teardown

Dev stopped by its pid (SIGTERM), the stand-ins and the replica's helper
apps stopped and unregistered, `/tmp/aij-lab-r05-idle` and the scratch
copy of the build removed; no "AI Juggler Dev" process remained.
