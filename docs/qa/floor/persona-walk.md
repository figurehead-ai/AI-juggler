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

- **The build:** the release candidate's AI Juggler Dev-edition build of the candidate commit (a release build, not `tauri dev`), started with `HOME` set to the lab, by pid only (never ⌘⇧M, never System Events: the installed app has the same process name).
- **The lab:** `scripts/qa/lab_scenario.sh setup FL1 --home /tmp/aij-lab-fl1` (`docs/qa/scenarios/FL1-the-floor.toml`): three subscriptions (Work, Personal, Client) and twelve Apps; one session asking, one working, one crashed, one probably stuck, one at its limit, one idle; Personal at 92 % used; 300 ended sessions. A watcher for a keychain dialog runs throughout, and `AIJ_NO_KEYCHAIN=1` is set.
- **The live part** (flows F3, F4), on the Lab CLI App (ADR-0027 7b) only, in a throwaway project under `~/lab/`:
  - a swarm: one session asked to use three subagents at once on a small task;
  - an App working the Mac hard: a bounded CPU load in that session's own tree (`trap` and a 120 s lifetime; afterwards `pgrep -x yes` must be 0).
- **What isn't walked by an agent, and is said:** macOS permission prompts, the menu bar, notifications, a second display.

## The driver (the testers' only command)

The Floor is a native window, so testers can't use the browser. They get exactly one command, `scripts/qa/floor_drive` (`floor_drive.swift`), and the firewall check voids any run whose transcript holds another shell command, a file read or a search.

| Verb | Does |
|---|---|
| `shot` | A capture of the Floor window (by the Dev build's pid), saved to the run's folder; its path is printed |
| `click X Y`, `drag X1 Y1 X2 Y2`, `scroll X Y DY`, `move X Y` | A pointer event at window points, posted to that pid only |
| `key NAME [mods]`, `type TEXT` | A key or text, posted to that pid only |
| `list` | Switches the main window to the Floor list and captures it |

Every action prints the time until the window settles (the response the walk measures).

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

1. `scripts/qa/floor_drive` is written (it compiles; not yet tried on a Floor window): try each verb on a Dev build of the Floor (a Dev window, agreed with the 0.4.1 session).
2. Run the briefs through the leak lint (`docs/qa/persona-validation.md`).
3. A pilot: one tester run of F1 on the Dev build, to check the driver and the firewall check, not to score.
