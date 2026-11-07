# The Floor, walked by personas on the release candidate (AJ-E3514)

The sheet for AJ-E3514's walk, in E33's form (`docs/qa/persona-validation.md`): fresh tester agents walk the Floor as the personas, a critic checks every claim against the captures, the Builder (this session) triages and fixes. It runs on the **release candidate's build**, not a debug build, once the main session names the candidate. Prepared 2026-11-07; nothing here has run yet.

## Roles and models (the maintainer's model rule, 2026-11-07)

| Role | Model | Sees | Does | Never |
|---|---|---|---|---|
| Builder | Opus (this session) | everything | sets up the lab, writes the briefs, triages, fixes, re-walks | scores, or lowers a score or severity |
| Tester | Sonnet, a fresh agent per run (×3 per flow) | the persona's Brief, one flow's Goal, and the Floor through the driver | walks, records each step, scores | reads files, code, docs, memory or earlier runs |
| Critic | Opus, a fresh agent | the run's step log and captures | checks each claim against the pixels; classifies each failure (the app's, the agent's, the setup's); rates severity 0–4 | sees the code |
| Maintainer | — | the app | calibration, after the release (not asked during 0.4.x) | — |

## Where it runs

- **The build:** the release candidate's commit built in release with the `floor-harness` feature (`scripts/floor_harness.py build`; it runs as AI Juggler Dev), started with `HOME` set to the lab, `AIJ_NO_KEYCHAIN=1` and `AIJ_FLOOR_INPUT` set to a new folder, by pid only (never ⌘⇧M, never System Events: the installed app has the same process name).
- **The lab:** `scripts/qa/lab_scenario.sh setup FL1 --home /tmp/aij-lab-fl1` (`docs/qa/scenarios/FL1-the-floor.toml`): three subscriptions (Work, Personal, Client) and twelve Apps; one session asking, one working, one crashed, one probably stuck, one at its limit, one idle; Personal at 92 % used; 300 ended sessions. A watcher for a keychain dialog runs throughout, and `AIJ_NO_KEYCHAIN=1` is set.
- **The live part** (flows F3, F4), on the Lab CLI App (ADR-0027 7b) only, in a throwaway project under `~/lab/`:
  - a swarm: one session asked to use three subagents at once on a small task;
  - an App working the Mac hard: a bounded CPU load in that session's own tree (`trap` and a 120 s lifetime; afterwards `pgrep -x yes` must be 0).
- **What isn't walked by an agent, and is said:** macOS permission prompts, the menu bar, notifications, a second display.

## The driver (the testers' only command)

The Floor is a native window, so testers can't use the browser. They get exactly one command, `scripts/qa/floor_drive` (`floor_drive.swift`), and the firewall check voids any run whose transcript holds another shell command, a file read or a search. It never moves the real pointer, so the Mac stays usable during the runs.

- **The Floor's window:** `open`, `shot`, `click`, `move`, `drag`, `rdrag` (turn and tilt), `scroll` (pan), `zoom`, `key`. Posted system events don't reach the Floor, so these go through its test input (`crates/juggler-app/src/features/floor/drive.rs`): the same inputs its event monitor makes, into the same render thread, over an owner-only socket. That input exists only in a build with the `floor-harness` feature, and only when `AIJ_FLOOR_INPUT` names a new folder and the keychain is off. No shipped build has it: `scripts/floor_harness_check.sh` (in CI and the preflight) checks the feature graph and the release workflows, and `scripts/release_seams_check.sh` checks the shipped binary for its variables. Each action prints the time to the first frame that shows its result (the Floor's frame counter, not stillness: its people never hold still).
- **The main window:** `main shot`, `main click` (presses the element under the point, or puts the cursor in a text box), `main type`, `main scroll`, through accessibility by pid. Each prints the time until the window changed.
- **The system's own input:** one short pass of about 5 minutes moves the real pointer (tabs, a door, turn and tilt, a key), so the OS input path is exercised once. It runs when no other session is driving the Mac. It uses events at the system level with AI Juggler Dev brought forward by its pid. System Events' `click at` isn't used, because it targets processes by name and the installed app has the same one.

## Personas and flows

The personas are E33's (`docs/qa/0.4-walks/personas/`): Maya (primary, ×2), Devon, Priya. A tester gets the persona's **Brief** and one flow's **Goal**, nothing else. The Goals below are in the persona's words; the success state, the abandon rule and the shortest path stay with the Builder.

| # | Persona | Goal (the tester's words) | Success, observable | Abandon if |
|---|---|---|---|---|
| F1 | Maya | "Open the office view and tell me who's waiting on me. Answer them." | The asking session is opened and answered | 2 minutes without knowing who waits |
| F2 | Devon | "Something crashed and something's eating my Mac. Find both." | The crashed session and the overloaded App are each opened | Either not found in 3 minutes |
| F3 | Devon | "One of my sessions spun up a bunch of helpers. Where are they, and what are they doing?" | The session with the helpers is found and its helpers' work is seen | Can't tell which session has them |
| F4 | Maya | "Which of my accounts is about to run dry?" | Personal is named, with how much is left | Ends up guessing |
| F5 | Priya | "I just added my client's app. Where did it go, and did everything else move?" | The new App's area is found; the others are where they were | Can't find the new one |
| F6 | Devon | "Get a better angle on this floor, then put it back how it was." | The view turns and tilts, then resets | Can't get it back |
| F7 | Devon | "Too many apps on one floor. Spread them out." | Apps per floor changed; floors made; areas in the same order | — |
| F8 | Maya | "I'm bored of how my office looks. Give me a new one without losing anything." | Re-create done; Apps, sessions and areas' order kept | Worried she lost something |
| F9 | Priya | "I can't use the 3D view today. Do the waiting-and-crash check with words only." | F1's and F2's success from the Floor list alone | Needs the 3D view |

Every step of every flow is scored as E33 says (success, ease, value, clarity, feeling, response). R3's hand walk (every element of the Floor and its pane clicked, its response measured) is the Builder's, after the persona runs, with the same driver.

## The bar (AJ-E3514 R2)

A flow passes as E33's bar says. In addition, AJ-E3514's own: **every step scored under 4 on success, value, clarity or feeling (on E33's scales mapped to 5) goes to the maintainer's list** (`docs/qa/0.4-completion.md`, put to them once at release time), and a bug is fixed before the release. The response budgets are SPEC §9.7's, read during the walk.

## Evidence

- Runs: `docs/qa/floor/persona-walk/<flow>-run<n>.md` (step records, captures beside them, opened by eye).
- Issues: `docs/qa/floor/persona-walk/issues.md`.
- The task's manifest: `docs/qa/evidence/AJ-E3514/manifest.md`.

## Before the candidate

1. `scripts/qa/floor_drive` was tried on 2026-11-07 (the release build of the floor target, lab FL1, `AIJ_NO_KEYCHAIN=1`, the keychain guard on; no dialog). `shot` works. **Input doesn't:** events posted to the pid (`CGEvent.postToPid`) reach neither the main window's web view nor the Floor's window, with the app in front, an event source, and the window named in the event; the process may post (`CGPreflightPostEventAccess` true). Accessibility by pid works for the web view (the element under a point, pressed: that opened the Floor), but the Floor's window shows nothing to accessibility beyond its title bar. Also: the Floor never holds still (its people move), so "settled" must be measured otherwise (the time to a changed frame, or the app's own frame counter). The way forward is the main session's call (see the milestone report).
2. ~~Run the briefs through the leak lint~~ Done 2026-11-07: the nine Goals and the three persona Briefs hold none of the leak list's words (`docs/qa/persona-validation.md`). Run it again on any Goal changed after the pilot.
3. ~~A pilot~~ Done 2026-11-07 (`persona-walk/pilot-2026-11-07.md`): the driver and the firewall check work; the real pointer's short pass is done. It found a bug candidate (the main window stays off screen behind the Floor after a door) to reproduce on the candidate first.

## The run on the release candidate

Once the main session names the candidate. One Dev slot of about 3 hours, agreed with the 0.4.1 session; nothing else drives the Mac during step 3.

1. **Build** the candidate's commit with the `floor-harness` feature as AI Juggler Dev.app (`scripts/floor_harness.py build --app`, the floor target, about 15 minutes, in the background). It must be the bundle: the bare binary leaves the main window off screen behind the Floor. `scripts/floor_harness_check.sh` passes on the commit. For F3 and F4's live sessions, `juggler-agent` (release, the same commit) goes beside the bundle's `juggler-app`, as the Dev build has it, or those sessions carry no hooks.
2. **Lab:** `lab_scenario.sh setup FL1 --home /tmp/aij-lab-fl1`. Start the bundle's `Contents/MacOS/juggler-app` with `HOME` set to the lab, `AIJ_NO_KEYCHAIN=1` and `AIJ_FLOOR_INPUT=/tmp/aij-floor-in` (a short path); the keychain guard runs first. To stop it: the `--aij-watch` child first (or the watchdog relaunches it), then the app, then the guard.
3. **First checks and the real-pointer pass, 5 minutes:** open the Floor; the main window stays on screen beside it. A door (a person; Enter on a selected area) brings the main window forward at that page. The title and the Floor list say Home's Needs you number. With the real pointer: two tabs, a turn, H and a door. If any of these fails, stop and fix it before any persona run.
4. **Persona runs:** F1–F9, three fresh Sonnet testers each (27 runs, one at a time, about 2 minutes each), prompts as `persona-walk/briefs.md` with this run's pid and socket. Between runs: Home in the main window, the Floor closed, H. The lab is set up again (step 2) before F5 and F8, which change the office. F3 and F4's live part (the swarm, an App working the Mac) runs on the Lab CLI App only; its load is bounded (`trap`, 120 s) and `pgrep -x yes` is 0 after.
5. **After each run:** the firewall check on the transcript (Bash only `floor_drive`, Reads only its pictures, the report hand-back allowed); the leak list on its words; captures opened by eye; then an Opus critic on the run (`briefs.md`).
6. **The Builder's hand walk (R3):** every element of the Floor and its pane, clicked through the driver, each response written down.
7. **Triage:** bugs fixed at once, and their flow walked again by a new tester. Every step under 4 on success, value, clarity or feeling goes to `docs/qa/0.4-completion.md` for the maintainer, once, at release. Then the AJ-E3514 manifest.
8. **Teardown:** the app (watcher first), the lab, the socket folder; "Dev slot back" to the 0.4.1 session and the main session.

The pilot's findings are fixed (ace8790b0, 34b7b69bc; checked live 2026-11-07): the Floor's title and list say Needs you's one count, then the sessions waiting; an area says what waits ("1 asking · 2 working"); the flags and hover label are readable at the opening view.
