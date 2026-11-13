# AJ-E805 — idle CPU, 0.4.5

2026-11-13, reviewer claude-opus. Machine: the reference Mac (Apple M5
Ultra, 30 cores, macOS 27, about 850 processes), shared with other work
while measured.

## The finding

The 0.5 session measured the installed 0.4.3 beta (read-only `ps`) at
about 20 % of one core idle (19–23 %), and CI's 0.4.3 release candidate on
the synthetic fixture (10 Accounts, 50 Instances, none live) at 5.7 %. The
budget is under 2 % with 10 live sessions. Read-only `ps` on the installed
beta (2026-11-13) showed two more things:

- with its window on screen, its own WebKit processes were busy beside
  `juggler-app` (12–18 %): the GPU process at about 11 %, the web content
  process at about 12 %. That is the window drawing, in neither
  `juggler-app`'s figure nor the backend-only proof (0.39 %);
- with no window on screen, `juggler-app` alone still read 17 %, its WebKit
  processes 2 % and 1 %. This Mac's own Claude Code profiles hold about
  5,400 transcripts (a count of files, nothing read), the lab 302: the
  backend's per-file work is what grows with them.

## Method

- **Builds.** "Before": the Dev edition's release build
  (`scripts/build_dev_app.sh`) at 7b527eb83 (0.4.4 merged). "After": the
  same at b816837f2: this work (72e0a57ae, 17bad9b4a, 7596f947b) with the
  release branch's naming and test commits beside it, and df2caecfb, which
  a99736df8 reverts (it changed nothing measurable, see below). Symbols kept.
- **Lab.** A throwaway HOME, `scripts/qa/lab_scenario.sh setup PD3 --home
  /tmp/aij-lab-idle --import`: the replica of a heavy Mac (seven Claude
  profiles, two Codex homes, 302 Claude transcripts, 13 repositories)
  brought in as 9 Accounts, plus PD3's 3 Accounts, 4 Projects, 52 ended
  sessions and 3 sessions: 12 Accounts, the AJ-E805 shape or heavier.
  **Grown lab:** the same with each Claude transcript cloned 15 times
  (`cp -c`), 4,832 in all, a real heavy Mac's count.
- **Live sessions,** outside AI Juggler, in the lab's projects: *working*,
  `scripts/replica.py live --claude 7 --codex 3 --no-desktop` (seven write
  a turn every 4–5 s); *quiet*, the same stand-ins resuming a conversation
  each and writing nothing.
- **Reading.** By pid only: the build's `juggler-app` and the web content
  and GPU processes it started. Cumulative CPU time (`ps -o time=`) at the
  start and end of 30 s intervals: each figure is the mean over 180 s
  (120 s on the grown lab), % of one core.
- **Conditions.** One launch per build per row group, the window first
  shown (never in front, nobody touching it) 75 s after launch (150 s on
  the grown lab, after its first read), then hidden (the app hidden by pid
  through System Events). On the grown lab the window's uncovered share was
  read from the window server: 100 % in both runs.
- **Profiles.** `sample <pid>` (1 ms) in each condition: busy samples per
  thread, waits left out.

## Results

| Lab, condition | Build | `juggler-app` | web content | GPU | All three |
|---|---|---|---|---|---|
| Shown, 0 live | before | 5.97 | 9.51 | 6.61 | **22.09** |
| | after | 1.62 | 0.59 | 0.00 | **2.21** |
| Hidden, 0 live | before | 2.01 | 0.36 | 0.04 | **2.41** |
| | after | 1.48 | 0.49 | 0.00 | **1.97** |
| Hidden, 10 working | before | 2.80 | 0.92 | 0.39 | **4.11** |
| | after | 2.20 | 1.10 | 0.39 | **3.69** |
| Shown, 10 working | before | 6.74 | 12.41 | 11.73 | **30.88** |
| | after | 2.17 | 1.13 | 0.43 | **3.73** |
| Hidden, 10 quiet | before | 3.23 | 1.07 | 0.31 | **4.61** |
| | after | 2.51 | 1.12 | 0.31 | **3.94** |
| **Grown**, shown, 0 live | before | 10.74 | 9.61 | 8.67 | **29.02** |
| | after | 3.83 | 0.70 | 0.06 | **4.59** |
| **Grown**, hidden, 0 live | before | 6.18 | 0.53 | 0.03 | **6.74** |
| | after | 3.95 | 0.74 | 0.00 | **4.69** |

`juggler-app`'s 30 s readings (min–max): before shown 5.25–6.33, hidden
1.73–2.54; after shown 1.47–2.00, hidden 1.43–1.54; grown before hidden
5.86–6.51, after 3.69–4.21. The quiet runs' "shown" halves are left out:
the before build's web view didn't draw there (1.2 % with the breathing
dot on screen), so its window wasn't seen.

## The causes

1. **The window drew every frame, for ever** (UI; most of the cost while
   it shows). The header's This Mac dot (`MachineStatusLine`, class
   `.dot-live`) breathed with an endless box-shadow animation, so WebKit
   repainted and committed a frame every vsync. Evidence: before, shown vs
   hidden, 22.1 % vs 2.4 %; the shown profile: the app's `CVDisplayLink`
   thread (`RemoteLayerTreeDisplayLinkClient::displayLinkFired`) 228 busy
   samples in 20 s, the JavaScriptCore allocator's scavenger 703
   (`pas_thread_local_cache_for_all`), the main thread 178. After: no
   display-link samples, scavenger 35, main thread 8; on the grown lab,
   window 100 % uncovered, web content 9.6 → 0.7 %, GPU 8.7 → 0.06 %.
   `ui/tests/visual/idle.spec.ts` found more endless loops at rest once it
   looked: app icons not yet read (out of view) pulsing, a live session
   page's Live mark and switcher dot pulsing, the terminal cursor's blink.
2. **The usage pipeline read everything every second** (backend; what
   grows with history). Every second it listed each Claude account's
   `projects` tree, looked at every transcript's size and wrote its
   offsets blob (62 KB on the lab) back to storage. Evidence, hidden, the
   pipeline thread (`usage_pipeline::ingest_pass` → `walk` →
   `opendir`/`getdirentries`/`stat`): lab 288 → 34 busy samples in 30 s;
   grown lab 2,076 → 191 in 20 s, the before build's largest by far.
3. **Readers of the live stats ran a collector tick each** (backend).
   `live_stats_of` called `LiveStatsCollector::refresh`, a whole tick, from
   the 5 s resources emission, the 15 s heuristics and every
   `instances_list`/`projects_list` the window asks for: the process table
   read again (`__sysctl`/`__proc_info`), the zombies' `ps`, the reconcile
   and the ports. Evidence, lab hidden: the `resources` thread 89 → 50
   busy samples in 30 s (`emit_resources → live_stats_of → refresh → tick`),
   `heuristics` 10 → under 1.

## The fixes

| Commit | What |
|---|---|
| 72e0a57ae | `live_stats_of` takes the 2 s tick's stats while under 3 s old; a caller with no tick running still samples |
| 17bad9b4a | `usage_pipeline::gate`: a pass runs on file events under the trees it reads (a recursive `FolderWake::trees`), on Claude's limit-record cadence (15 s: the bound for a late or lost event) and the stores' own; the offsets blob is written only when it changed; where events can't be heard, the old cadence |
| 7596f947b | `.dot-live` breathes three times, then rests (the This Mac dot again when its level changes); the session page's Live mark and the switcher's dot likewise; an app icon out of view waits still; a terminal's cursor blinks for 10 s after its last key or output |
| df2caecfb, reverted by a99736df8 | a process table kept between presence reads: no change measured (52 vs 49 busy samples in 30 s); sysinfo reads every process's arguments on every refresh whatever it is asked |

Freshness kept: a write under a watched tree is read at the next second
as before; a lost event delays a usage reading by 15 s at most; the live
stats readers' data is at most one 2 s tick old, as the tick made it.
Presence (ADR-0032) is unchanged at HEAD: its process reads are as before.

## The guards

- `ui/tests/visual/idle.spec.ts` (every Playwright run): Home, and a live
  session's page with its terminal, at rest at scale with motion allowed,
  run no animation that never ends; the This Mac dot breathes at most
  three times. It fails with the old endless breathe (checked).
- `usage_pipeline::gate::tests::a_quiet_mac_runs_a_pass_only_on_the_records_cadence`:
  a minute with nothing written runs 4 passes (it ran 60).
- `read_model::tests::a_reader_takes_the_ticks_stats_without_sampling_again`:
  five readers after a tick run no tick.

## Open

- **The conversation list's 15 s pass, at scale** (the next cause). On
  the grown lab, hidden, after: the `conversations` thread took 1,087 busy
  samples in 20 s (`conversations::refresh_with` →
  `claude_conversations::conversation_files`: an `is_file` and a
  `metadata` per transcript, every 15 s), the largest left; on the small
  lab, 47. Same shape as cause 2; its fix is the same gate, but the
  folders to watch are each adapter's to say (no adapter names its
  conversation roots yet), so it needs that trait method.
- **Under 2 % with 10 live:** `juggler-app` reads 2.2 % with 10 working
  sessions and 2.5 % hidden with 10 quiet ones (1.5 % with none). The
  rest is the 2 s process collector (its listing every 4 s, the full read
  with arguments every 10 s, the zombies' `ps`; 0.4–0.8 %), presence's
  5 s full read (ADR-0032), and, while sessions work, the window's list
  reads on each change event. A cheaper process listing than sysinfo's
  (which reads every process's arguments every time) is unsafe code in an
  audited crate: a decision of its own.
- **Not measured:** the installed beta and the real Mac's own setup beyond
  read-only `ps` (no profile of them); IPC calls per minute from the
  window (a release build's web view can't be inspected; the read-only
  bridge's debug backend wasn't run); the Floor's renderer (its window
  wasn't opened; it draws only while open).
