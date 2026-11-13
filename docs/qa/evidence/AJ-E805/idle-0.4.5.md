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
| 17bad9b4a, a5e47246c | `usage_pipeline::gate`: a pass runs on file events under the accounts' folders and AI Juggler's spools (a recursive `FolderWake::trees`), on Claude's limit-record cadence (15 s: the bound for a late or lost event) and, while an account has one, the stores' own; the offsets blob is written only when it changed; where events can't be heard, the old cadence. a5e47246c makes the gate tool-agnostic (ADR-0019): it watches each account's folder rather than tool-named subfolders and learns from the pipeline whether a store exists; measured with 17bad9b4a's version, the same at rest |
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

## Follow-up, the same day: the process table, the conversation list, the window's reads

The first round left the budget unmet: under 2 % of one core for the app,
its web content and its GPU process together, with 10 live sessions.

### Method

As above, with these differences:

- **Builds.** "Before": the Dev edition's release build at 41a202078 (the
  first round, a5e47246c included). "After": the same at 91897b9a2 (this
  work; after it, docs, f0929c546's refactor and ba4f8c2c3's presence fix
  only), and where a row says so at bc4ee5a90 (the same less 3b37420b6's
  window change) or fda9495f1. Symbols kept.
- **Labs.** PD3 with the replica, imported, set up fresh: 342 transcripts,
  12 Accounts. **Grown lab:** a second such lab, each Claude transcript
  cloned 15 times (`cp -c`), 4,848 in all. Each build ran on the same lab
  folders, before first.
- **Launch.** `open -g -n --env HOME=<lab>`: behind the front app, never
  activated, nobody touching it; then by pid only. The web content, GPU and
  networking processes are the WebKit processes whose responsible pid is
  the app's (`responsibility_get_pid_responsible_for_pid`).
- **Reading.** `ps -o time=` every 30 s; the mean over 180 s (120 s on the
  grown lab), the window shown 75 s after launch (150 s grown), then hidden
  through System Events by pid. Before each reading, the window's uncovered
  share from the window server (98–100 %) and the load average (4–11: the
  Mac ran other builds throughout, and This Mac read "Under pressure" for
  part of the before runs, which moves the shown figures by a few tenths).
- **Live sessions.** *Working*: `replica.py live --claude 7 --codex 3
  --no-desktop`. *Quiet*: ten stand-ins each resuming an existing
  conversation of its Account and writing nothing (`--idle`). On the grown
  lab, 10 working too, hidden (the first round didn't measure it).
- **Profiles.** `sample <pid> 20 1` after each reading; busy samples per
  thread, waits left out (`mach_msg`, `__psynch_cvwait`, `kevent`,
  `semaphore_timedwait`, sleeps, idle workqueue threads).
- **IPC.** A measurement-only build of the before commit (never committed)
  counting each command the window invoked, per minute, on the grown lab.

### The profile, ranked (before)

Busy samples in 20 s at 1 ms, by thread and what it ran. Small lab hidden
at rest (401 in all) and with 10 working (311); grown lab hidden at rest
(727) and with 10 working (834).

| Cause | Small, rest | Small, 10 working | Grown, rest | Grown, 10 working |
|---|---|---|---|---|
| **The process table** (the collector's thread and presence's): the 2 s collector's listing every 4 s and full scan every 10 s, presence's look every 5 s, the live view's every 60 s, each `sysinfo` reading every process (about 850) with its arguments (`KERN_PROCARGS2` twice: 72 % of a read) | 216 | 154 | 226 | 166 |
| **Copies of the whole conversation index**: the live refresh's `all()` every 2 s, Needs you's working list and session titles every 5 s | 8 | 10 | 186 | 138 |
| **The conversation list's 15 s pass** (a listing and two `stat`s per transcript) | 8 | 18 | 68 | 117 |
| **The window's list reads on every change**, hidden or not (IPC workers; the web content 0.9–1.4 % and GPU 0.3–0.4 % while hidden) | — | ~20 | — | ~96 |
| **Needs you's 5 s read** besides the copies (sessions' rows, their branches, Accounts, budgets) | 26 | 21 | 76 | 63 |
| **The usage pipeline** (the records' 15 s cadence walks every transcript; under load, every event did) | 14 | 14 | 71 | 26 |
| JavaScriptCore's allocator scavenger, in the app process | 78 | 11 | 20 | — |
| The tool-version check, once a day (it loads the system's trust store) | — | — | — | 97 |

**IPC per minute (before, grown lab).** At rest: shown 16–17
(`fleet_remote` 10–11, `link_status` 3, `alerts_tick` 2: the window's own
polls, already stopped while hidden), hidden 2 (`alerts_tick`, by design).
With 10 working: hidden 74–90 and shown 88–89, all on change events:
`relations_counts` 24–36, `conversations_list` 8–10, and the shell's lists
(`accounts_list`, `instances_list`, `launch_recipes_list`, `projects_list`,
`switch_back_suggestions`) 8 each. No UI timer is a cost at rest; the reads
on change events are, with sessions at work, and they ran while hidden.

### The fixes

| Commit | What | Guard |
|---|---|---|
| a040caa4a | **D-110**, `juggler-proc-list`: one `sysctl(KERN_PROC_ALL)` (the call `ps` makes) lists every process's pid, parent, start time, name and zombie state, never its arguments; the record layout checks itself on every read. `procs::table` (presence's look, the live view, the places) keeps its table: arguments read once per process (same pid, start and name), parents as listed now, everything again every 60 s | `procs::tests::a_read_after_the_first_reads_only_what_is_new`; the shim's tests against `ps` and a real zombie |
| 00ba04023 | The collector's listing reads only the watched trees' processes (it read all of them every 4 s); zombies from the same listing (no `/bin/ps`) | `tests/observed.rs::a_listing_reads_the_watched_trees_not_every_process` |
| bc4ee5a90 | The full scan reads whole only new processes and ours; This Mac's other apps' memory from `juggler_proc_list::resident` (D-110: four audited `unsafe` blocks in all); every process whole once a minute | `tests/observed.rs::a_full_scan_reads_the_new_processes_not_every_one` |
| fda9495f1 | `ToolTranscripts::conversation_roots` (ADR-0019; Claude Code's `projects`, Codex's, Kimi's and Grok's `sessions`) and `conversations::gate` on their file events: a known file that changed is read alone, anything else new lists its Account again, an Account whose tool names no folder every pass, everything on the first pass, a lost track and every 5 min; `conversations::pass` split from `mod.rs` | `gate::tests::a_pass_reads_what_changed_and_sweeps_rarely`, `pass::tests::a_scoped_pass_reads_only_what_it_names`, each adapter's roots hold its files |
| c40005044, 499f805fc | `ConversationIndex::all_where`: the live refresh, the working list (`conversations_list`, Needs you), the session titles and the first-read check copy only what they use | `commands::tests::the_working_set_from_its_candidates_is_the_whole_lists` |
| 0f8dac45f | The usage pipeline: a pass on file events reads only the files they named (spelled as the Accounts' folders are); the records' 15 s cadence still walks everything | `usage_pipeline::gate::tests::a_pass_on_events_reads_the_files_they_named` |
| 8fed5f260 | The window: a sessions, conversations or places event heard while hidden reloads once, when it shows (`whileShown`) | `shown-window.test.ts` |
| 3b37420b6 | The shell's lists keep their identity when the answer is equal: no redraw | `use-app-data.test.ts` (fails without it) |
| f0929c546 | No copied code: `folder_wake::Heard` shared by both gates, one sort, one targeted read | the debt baseline |
| ba4f8c2c3 | Presence: a process the kernel is exiting (`P_WEXIT`) is gone at once (a quit showed 2.2 s late); another user's processes stay in the table as before | `procs::tests::a_process_started_since_is_read_with_its_arguments`; the presence check below |

### Results

| Lab, condition | Build | `juggler-app` | web content | GPU | All three |
|---|---|---|---|---|---|
| PD3 + replica, shown, 0 live | before | 1.69 | 1.23 | 0.74 | **3.66** |
| | after (91897b9a2) | 0.83 | 0.59 | 0.03 | **1.45** |
| Hidden, 0 live | before | 1.49 | 0.89 | 0.26 | **2.63** |
| | after (91897b9a2) | 0.82 | 0.40 | 0.01 | **1.22** |
| Hidden, 10 working | before | 2.09 | 1.37 | 0.44 | **3.91** |
| | after (bc4ee5a90) | 0.99 | 0.35 | 0.00 | **1.34** |
| | after (91897b9a2), shown first | 1.14 | 0.64 | 0.32 | **2.11** |
| Shown, 10 working | before | 2.20 | 1.80 | 1.00 | **5.01** |
| | after (bc4ee5a90) | 0.97 | 0.76 | 0.28 | **2.01** |
| | after (91897b9a2) | 1.08 | 0.93 | 0.32 | **2.32** |
| Hidden, 10 quiet | before | 1.61 | 0.85 | 0.33 | **2.79** |
| | after (bc4ee5a90), 5 of 6 readings | 0.96 | 0.37 | 0.00 | **1.33** |
| | after (91897b9a2), shown first | 1.20 | 0.70 | 0.44 | **2.34** |
| **Grown**, shown, 0 live | before | 2.50 | 0.61 | 0.02 | **3.12** |
| | after (fda9495f1, the first build after) | 1.32 | 0.81 | 0.02 | **2.15** |
| | after (91897b9a2) | 1.11 | 1.00 | 0.21 | **2.31** |
| **Grown**, hidden, 0 live | before | 2.87 | 0.42 | 0.01 | **3.30** |
| | after (91897b9a2) | 1.16 | 0.45 | 0.00 | **1.60** |
| **Grown**, hidden, 10 working | before | 3.99 | 1.39 | 0.32 | **5.71** |
| | after (91897b9a2) | 1.45 | 0.41 | 0.01 | **1.86** |

- **`juggler-app` itself** reads 0.8–1.5 % in every condition after
  (1.5–4.0 % before): about half at rest, a third or less with sessions
  at work and on the grown lab.
- **The budget (under 2 % for the three, 10 live sessions)** is met hidden
  with 10 working or quiet when the window was hidden throughout (1.3 %),
  at rest (1.2–1.6 %), and on the grown lab hidden with 10 working
  (1.9 %). It is **not met with the window shown and 10 sessions working:
  2.0–2.3 %** (5.0 before; a third run at load 12–16, not in the table,
  read 3.1), and not after the window was shown with sessions at work
  and then hidden (2.1–2.3 %): the web content and GPU process stay at
  0.6–0.7 % and 0.3–0.4 % hidden. Pausing every animation while hidden was
  tried, measured no change at load 12–16 and was dropped; that cause is
  open.
- **The shared Mac moves these figures:** the window opened on the main
  display in some runs and on the second display in others (the build
  restores it where it was; `uncovered` names the place), the load average
  ran 4–16, and This Mac read "Under pressure" for part of the before
  runs. The bc4ee5a90 and 91897b9a2 rows differ in that way, not in code
  that runs while hidden (3b37420b6 changes only what the window draws).
- **Profiles after** (busy samples in 20 s; before in brackets): small lab
  hidden at rest 151–157 (401), its largest the collector (65–70: the full
  scan's own reads and the observed trees' listing) and Needs you's 5 s
  read (31); grown lab hidden at rest 176 (727), with 10 working 301 (834).
  Presence's look fell from 56 to 9–11, the conversation list's pass from
  68–117 to under 5 at rest, the live refresh's copies from 106 to under
  5.

### Presence (ADR-0032)

`scripts/qa/presence_check.py` on the PL1 lab (`lab_scenario.sh setup
PL1`: two Claude app profiles started and quit with `lab_apps.py`, the
real Claude.app on lab folders, never touched), with the final build
(ba4f8c2c3), twice: **PASS** before the start, after it and after the
quit, each time. How long after `ps` showed the change the rail did (the
accessibility tree polled every ~0.1 s, one read included): **launch
0.51 s and 0.38 s, quit 0.89 s and 0.36 s** (ADR-0032: within 2 s). The
before build: 0.85 s and 0.13 s.

The build before ba4f8c2c3 (91897b9a2) showed a quit 2.17 s late: the
system's quit notification asks for a look at once, and the listing still
named the process while the kernel was exiting it. ba4f8c2c3 reads an
exiting process (`P_WEXIT`) as gone. One earlier run of that build also
failed outright (Open now empty for a minute with both profiles running)
while another session was using the Dev edition's bundle id for a
rehearsal; it didn't recur in the three runs after.

### Open

- **Needs you's 5 s read** is now the backend's largest cost at rest
  (20–50 busy samples in 20 s): the session rows with their branches
  (`.git/HEAD` per session), the Accounts, budgets, messages and tasks,
  window or not (D-49). Worked out on change rather than every 5 s, it
  would go.
- **The usage pipeline's 15 s records cadence** still walks every
  transcript (57 busy samples in 20 s on the grown lab at rest, about
  0.3 % of a core at 4,800 transcripts): the bound the first round chose
  for a late or lost event. Reading the Claude app's records alone on that
  cadence, and the transcripts on events with a longer sweep, would remove
  it.
- **The observed tools' trees** (the Claude app's ~300 processes on a busy
  Mac) are still read through `sysinfo` at each listing, arguments
  included; a CPU-time read beside `resident` in the shim would make them
  cheap too. The Processes view's graph is also worked out every 2 s
  whether or not the page is open (12–14 samples).
- **With sessions at work and the window shown**, the window still reads
  its lists on each change (about 80 calls a minute before; not counted
  after); it no longer redraws when an answer is equal (3b37420b6), but the
  reads aren't coalesced (`relations_counts` the most). The projects
  backfill also runs after every pass that changed a file (31 samples on
  the grown lab with 10 working).
- JavaScriptCore's allocator scavenger in the app process varies between
  runs (7–78 samples) and wasn't traced to a cause.
- **Not measured:** the IPC counts on the after build (the hidden window's
  reads are held by `shown-window.test.ts`, and its web content fell from
  0.9–1.4 % to 0.3–0.4 % hidden); the installed beta beyond `ps`.
- **D-110's number:** first recorded as D-108 (the commit messages from
  a040caa4a to ba4f8c2c3 say so); release/0.5 had already taken D-104 to
  D-109, so the shim is D-110, and this branch's log skips D-104 to D-109.
- **The shared Mac quit three lab runs:** another process pressed the hidden
  lab window's close button through the accessibility API (the system log:
  `performClick:` then `windowShouldClose:`, nobody at the window), and
  with no session of its own running the build quit as asked (AJ-E1503).
  Those runs' readings after that moment were dropped and the conditions
  run again.

## Round 3, the same day: the hidden window's processes, the shown window's page, the walks

Round 2 left two conditions over the budget: shown with 10 working
(2.0–2.3 %) and hidden after a show (2.1–2.3 %, the web content and GPU
process busy with nothing on screen, cause unknown).

### Method

As round 2, with these differences:

- **Builds.** "Before": a476f8e5b, built at 8c3eb0430 (its D-110 rename
  only: the same code). "After": 4d7c215d5. The Dev edition's release
  build, copied out of the shared target before it was measured.
- **Lab.** PD3 with the replica, imported, set up fresh (342 transcripts,
  12 Accounts). *Working*: `replica.py live --claude 7 --codex 3
  --no-desktop`. *Quiet*: the same ten, each resuming an existing
  conversation of its Account and writing nothing (a copy of `replica.py`
  in /tmp with `--idle` for all, never committed).
- **Launch and reading.** `open -g -n --env HOME=<lab>`, by pid, quit by
  bundle id. Each run: 75 s, 180 s shown, the app hidden through System
  Events, 15 s, 180 s hidden; `ps -o time=` every 30 s. A B A B A B per
  condition, one build at a time. The load average was 9–71 (other
  builds on the Mac; the first pair at 45–71).
- **Window.** "Shown" is what WebKit counts as visible: the window on
  screen, uncovered (93–100 %) or under another window (0 %: WebKit's page
  stays visible). `open -j` ("never shown") put the window on screen under
  another window in this series (0 % uncovered), so those rows are shown,
  covered; a page hidden throughout now does what a page hidden after a
  show does (below), which the hidden rows measure.
- **No `sample`.** On this build it needs the developer-tools
  authorization (an lldb attach raised that password prompt once; it was
  dismissed, no password entered). Instead: per-thread CPU from
  `proc_pidinfo(PROC_PIDTHREADINFO)` over 30–60 s, the system log of the
  build's pid (`log show --predicate 'processID == …'`, WebKit's own
  trace), and measurement-only builds (never committed) counting each
  command and event per 10 s and, in the page, timer, rAF and MessagePort
  callbacks, DOM mutations by the nearest named element, the running
  animations and the visibility state.

### What was found

1. **Close is quit.** The window's close button asks `quit::request`
   (`lib.rs`, `on_window_event`): AI Juggler quits, or shows the quit sheet
   while its own sessions are live; Cancel leaves the window shown. No
   window is left hidden by a close: hidden means ⌘H, minimised or on
   another Space.
2. **Shown, then hidden: every backend event woke three processes.** After
   the hide the page knew it was hidden (`visibilityState` hidden, 2 timer
   callbacks a minute, no rAF), but the backend's events kept coming: a
   Tauri event is a script the web view runs (`WebPage::
   runJavaScriptInFrameInScriptWorld`), and WebKit runs one as a
   foreground activity. In a hidden page each one raised the web content,
   GPU and networking processes to the foreground and dropped them back:
   with 10 working, hidden, about 60 scripts, 57 foreground assertions and
   340 throttle-state changes a minute; shown, none. Most were
   `processes://changed` (about 18 a minute: the graph moves on nearly
   every 2 s tick while sessions work) and `machine://this-mac` (12).
3. **Shown with 10 working: the page rewrote itself.** With nothing
   changing on screen, about 2,000 attribute writes a minute (the rail's
   roving marks, set again after every render of the rail) and 400 style
   writes (an App header's fitted words: each shown, measured and hidden
   again after every render, a layout and a repaint each); the whole app
   rendered again on each 5 s This Mac reading (the shell's state); the
   top bar's count took every `processes://changed`; each list was read
   again on every change.
4. **The backend's walks.** The conversation pass ran the whole projects
   backfill after any changed file (every conversation placed, every
   project written); the usage pipeline's 15 s records cadence listed
   every transcript.

### The fixes

| Commit | What | Guard |
|---|---|---|
| 58c3e6607 | `listenWhileShown` (`ui/src/lib/shown-window.ts`): while the window is hidden the page holds no listener for a change event, so Tauri runs nothing in it; when it shows it listens again, then reads what it missed. Sessions, their resource and quota ticks, conversations, places, This Mac, processes, the notification summary, messages, tasks, driven sessions; it replaces `whileShown` and the copied listen-and-dispose code. Events that ask the window to act (quit sheet, go to, open) and terminal output are heard as before | `shown-window.test.ts` (no listener while hidden; one listener and one catch-up after a hide and show) |
| 4b3e719e2 | `coalesced()`: a list read on every change reads at once, then at most once a second; the relation counts, the shell's lists, the conversation list and a related list read through it (two copies of a 250 ms settle timer go) | `shown-window.test.ts`; `relation-counts.test.tsx` (the first change reads at once) |
| 31bbbd7d7 | The projects backfill runs when a conversation comes, goes or moves folder (`Placed`); lines appended move only their project's last use (`touch_last_used`) | `tests/conversations.rs::appended_lines_move_a_projects_last_use_and_only_new_conversations_backfill` (fails without it) |
| d5b64a04e | `rove()` writes a control's mark only when it lacks it; `useFitWhole` fits when the words change or the box changes width | `lib/roving.test.ts`, `place-header-fit.test.tsx` (both fail on the old code) |
| fe6ec46f9 | `processes://counts` (`AppEvent::ProcessCounts`): the top bar's count, sent only when a count moves; the graph's event has listeners only while the Processes page is open | `tests/processes.rs::the_top_bars_count_is_told_only_when_a_count_moves`; `lib/cleanup-count.test.tsx` (fails on the old wiring) |
| c86afe174 | `ThisMacProvider`: This Mac's reading draws what reads it, not the whole app | `lib/this-mac-provider.test.tsx` |
| 4d7c215d5 | The usage pipeline walks every transcript first, when asked, after a lost track or a change of the accounts watched, and every 5 min (`SWEEP_EVERY`); the records' cadence reads the records alone | `usage_pipeline::gate::tests::the_records_cadence_walks_nothing_and_the_sweep_walks_everything` |

The measurement-only builds after the first two fixes: hidden after a
show, the web content 0.73 → 0.05 % and the GPU process 0.32 → 0.00 %;
shown with 10 working, DOM writes about 2,400 → 70 a minute and the GPU
process 0.41 → 0.04 % (instrumented, so higher than the rows below).

### Results

Medians of three 180 s readings per build, interleaved (A B A B A B; the
last quiet pair ran again as B A after its stand-ins were stopped by
mistake mid-run, and those readings were dropped), % of one core.

| Live, condition | Build | `juggler-app` | web content | GPU | **All three** (min–max) | Load (1 min) |
|---|---|---|---|---|---|---|
| 10 working, shown | before | 1.27 | 0.95 | 0.31 | **2.53** (2.22–6.09) | 17–45 |
| | after | 1.24 | 0.67 | 0.10 | **2.03** (1.58–2.04) | 10–48 |
| 10 working, shown under another window (`open -j`, 0 % uncovered) | before | 1.52 | 1.02 | 0.36 | **2.92** (2.46–3.01) | 10–11 |
| | after | 1.33 | 0.70 | 0.11 | **2.14** (1.99–2.32) | 9–12 |
| 10 working, hidden after a show | before | 1.28 | 0.62 | 0.30 | **2.20** (1.92–3.23) | 12–71 |
| | after | 1.23 | 0.02 | 0.00 | **1.25** (1.15–1.42) | 9–22 |
| 10 quiet, shown | before | 1.07 | 0.79 | 0.34 | **2.20** (2.06–2.32) | 6–13 |
| | after | 0.93 | 0.42 | 0.01 | **1.62** (1.31–1.68) | 11–14 |
| 10 quiet, hidden after a show | before | 1.06 | 0.73 | 0.33 | **2.13** (1.87–2.17) | 5–11 |
| | after | 0.88 | 0.05 | 0.02 | **0.94** (0.94–0.96) | 10–17 |

- **The budget (under 2 % for the three, 10 live) is met hidden after a
  show** (1.25 working, 0.94 quiet; round 2's open cause) and **shown with
  10 quiet** (1.62). A window hidden from its start holds no listener
  either, so it reads as hidden after a show.
- **It is not met shown with 10 working: 2.03 % (2.14 under another
  window)**, against 2.53 and 2.92 before. The web content fell by a
  third and the GPU process by two thirds; `juggler-app` is now most of it
  (1.2–1.3 %), unchanged by these fixes but for the walks.
- **`juggler-app` by thread** (after, 10 working, shown, 45 s at load
  10–16; `proc_pidinfo`): the 2 s `processes` tick 0.33 %, the 5 s
  `resources` tick (Needs you, This Mac, the resource emission) 0.19,
  `live-sessions` 0.10, `keep-alive` 0.05, the main thread 0.04, the
  unnamed threads (the usage pipeline's among them) 0.04 (0.18–0.38
  before 4d7c215d5), IPC workers 0.03 (0.26 before 4b3e719e2), the 15 s
  `conversations` pass 0.11–0.36 when it falls in the window.
- **Two quits of hidden lab windows**, outside the table: a
  measurement-only run (11:31:45) and a first trial of the before build
  (12:30:48), each in its hidden half: dropped, not chased (round 2's
  cause).

### Presence (ADR-0032)

`scripts/qa/presence_check.py` on the PL1 lab with the final build
(4d7c215d5), three times: **PASS** before the start, after it and after
the quit, each time. How long after `ps` showed the change the rail's
Open now did (its accessibility tree read every ~0.1 s, one read
included): **launch 0.70, 0.21 and 0.83 s; quit 0.15 and 0.38 s** (the
first run's quit wasn't timed: the timer missed Open now leaving the rail;
its check passed). ADR-0032: within 2 s.

### Open

- **Shown with 10 working is still over the budget (2.0–2.1 %).** What is
  left, largest first:
  - the `processes` tick (0.33 %): the observed tools' trees read through
    `sysinfo` with their arguments, and the Processes graph built every
    2 s whether or not its page is open. Taking their CPU time from
    `proc_taskinfo` (`pti_total_user`/`pti_total_system`, in the record
    `resident` reads) needs Mach's timebase on Apple silicon (the fields
    are ticks, not nanoseconds): `mach_timebase_info`, a new unsafe call,
    or `hw.tbfrequency`. Not done: the brief allows no new unsafe block.
    Building the graph only while its page is open needs its counts (the
    top bar) and sightings (leftovers, Needs you) split from the rest.
  - Needs you's 5 s read (in the 0.19 % `resources` tick), window or not
    (D-49): change-driven, with each time-dependent input (snoozes ending,
    the working list's recency, sign-ins' 30 s check, the started words'
    day change, budgets' month) scheduled at its next deadline. Not done.
  - the shown page's renders on each change (the web content 0.4–0.7 %):
    each This Mac reading still draws every rail row (`useHeavyNote`
    reads the whole context), and each conversation change reads and
    draws the rail.
- **Not measured:** the grown lab; a window hidden from its start (`open
  -j` left it on screen under another window); the installed beta beyond
  `ps`.
