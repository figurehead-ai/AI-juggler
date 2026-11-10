# 0.4 release candidate: the plan, then the record (AJ-E3306, AJ-E3307)

The release candidate is walked whole, in one run, on the built app, before 0.4 ships (`docs/qa/0.4-validation-plan.md` §7–§8). This page is the run's plan first. Each row's result is filled in as it runs, and the summary for the maintainer ends it.

## 1. The build

- **What:** `release/0.4.1` at the commit the preflight passed, built with `just app`. That is a release-mode build of AI Juggler Dev (`ai.figurehead.juggler.dev`): unsigned, it never updates, and its data folder is its own, so the installed AI Juggler and its data are never touched. Since 4a9d1eb63 the recipe makes the Dev icon's `.icns`, so the bundle builds on a Mac without the release workflow.
- **Where it goes:** a folder of the run's own, not the shared bundle path another session may be running from: copy `$CARGO_TARGET_DIR/release/bundle/macos/AI Juggler Dev.app` (the shared target folder) to `$AJ_RC/AI Juggler Dev.app` and run it from there.
- **Never `tauri dev`**, and never a debug build for a row marked **rc**.
- **Version:** the build's `CFBundleShortVersionString` is recorded here before the first row (SemVer: 0.4.1).

## Who runs each step, and on which model (the maintainer's ruling, 2026-11-07)

Opus is for coding and judgement only. Every subagent is started with its model named:

| Work | Model |
|---|---|
| Writing or fixing code, design calls, root-causing a failure | Opus |
| Checking any FAIL, and any PASS on a requirement, that a cheaper model reports, before it's recorded or acted on (short) | Opus |
| Scenario rows that follow their sheet (§3), lab drives, the scripted first-run walks | sonnet |
| Persona tester runs (§4: every flow's three runs) | sonnet |
| The Critic and triage of the persona findings | Opus (on the findings only) |
| The heuristic sweep from screenshots, screenshot capture and checks | sonnet |
| Evidence-manifest drafts, the pairwise and L3 table runs, AJ-E808's cells | sonnet |
| Log scans, a PASS line per requirement in each manifest, counts, report formatting | haiku |

## 2. Before the first row

| Step | How | Result |
|---|---|---|
| Release scope | `python3 scripts/release_scope.py report 0.4`: every row proven or deferred by a D-number | |
| Preflight | `just preflight origin/main` (the fast local check, which stamps the commit for the push); the heavy gates run on GitHub through the release PR | |
| Re-baseline | the one visual re-baseline (notes: the side-session notes' `rebaseline-0.4.1-notes.md`), every capture opened by eye, the critique stamped after staging | |
| AJ-E3305 | the 24 families re-scored on that gallery (`docs/qa/0.4-walks/design.md`); lows fixed or sent to the maintainer | |
| The installed app | quit while a row needs the single-instance socket (`tests/second_launch.rs` hands over to it otherwise) | |

## 3. Every scenario again (AJ-E3306 req. 1)

Where each runs:
- **replica:** the replica HOME (`just replica`), the RC run with `HOME` set to it.
- **lab:** a scenario lab (`scripts/qa/lab_scenario.sh setup ID`), the RC run with `HOME` set to the lab, through the lab write mode where the row needs writes.
- **live-codex:** the real Codex sign-in where it is (ADR-0027 condition 7).
- **live-claude:** Claude Code in the Dev window on the real HOME, driven by pid, harmless commands in `~/lab/` (ADR-0027 condition 7b).
- **mock:** the design preview.

Every row below is driven by **sonnet** from its sheet. **Opus** checks each FAIL, and each PASS that closes a requirement, before it's recorded.

| ID | Scenario | Where | Steps | Who | Result |
|---|---|---|---|---|---|
| A1 | Open a Claude app profile | replica | 1 | agent | |
| A2 | Everything on one subscription | replica | 1 | agent | |
| A3 | From a session to its App and Account | replica | 2 | agent | |
| A4 | Projects an App was used for | replica | 1 | agent | |
| A5 | What an out-of-room Account affects | replica | 1 | agent | |
| A6 | Three Claude app profiles on two subscriptions | replica | 3 | agent | |
| A7 | Start Claude Code on the personal plan | replica, lab | 2 | agent | |
| A8 | Rename the work subscription | replica | 2 | agent | |
| A9 | Upgrade: one Account per sign-in | lab | 2 | agent | |
| A10 | A sign-in can't be read | lab | 1 | agent | |
| K1 | The chain | replica | 5 | agent | |
| K2 | Every page, every tab | replica | 6 | agent | |
| K3 | Notices land | lab | 2 | agent; the system notification itself is the maintainer's | |
| K4 | Pickers | replica | 2 | agent | |
| N1 | The usual | replica, lab | 2 | agent | |
| N2 | Change it | replica | 1 | agent | |
| N3 | The rare options | replica | 2 | agent | |
| N4 | First time | replica | 1 | agent | |
| N5 | The usual is out of room | replica | 1 | agent | |
| P1 | Writer and reviewer | lab, live-claude, live-codex | 9 | agent under 7b; the maintainer for any prompt | |
| P2 | Feature team | lab, live-claude | 6 | agent under 7b | |
| P3 | Across repositories | lab | 4 | agent | |
| P4 | A reviewer for everything | lab, live-codex | 4 | agent | |
| P5 | Rules by account | lab | 4 | agent | |
| P6 | A build Mac | mock | 2 | agent (pools across Macs are 0.5) | |
| P7 | Sealed client project | lab (step 3 live-claude) | 5 | agent | |
| P8 | Scouts | lab | 3 | agent | |
| P9 | Moving from 0.3 | lab | 3 | agent | |
| P10 | Set up by hand, first time | mock, lab | 3 | agent | |
| P11 | Overlap and deletion | lab | 3 | agent | |
| PF1 | Persona first run | lab (native window) | 1 | agent, by pid | |
| PM3 | Persona Maya: which account has room | lab | 1 | agent | |
| R1 | The runtime lab | lab, live-codex | 6 | agent (Claude steps 3–4 under 7b) | |
| X1 | A day | lab, rc, live-claude, live-codex | 6 | agent; the crash and the restart on the RC | |
| X2 | Load: a 50-member pool, 1 000+ ended sessions | lab, rc | 2 | agent | |
| NY1 | Archive takes a row out of Needs you at once | lab, rc | 3 | agent | |
| E3002-R3 | OpenCode and Grok Build with the agent tools ([sheet](E3002-R3.md)) | live (two test accounts, signed in by the maintainer), rc | 12 | agent under 7b; the maintainer for the two sign-ins | |

NY1, from the maintainer's report that a "stopped with an error" row stayed in Needs you until a restart:
1. Archive a session that stopped with an error. Time its row leaving Needs you and the bell count dropping. It must be at once, and at most one 5 s tick.
2. Unarchive it: the row comes back.
3. Archive a conversation in AI Juggler (another app's, finished, not looked at): its row leaves at once.

The code paths are fixed and tested (c81d79cd7, 8b36cf64f), but the cause of "until a restart" isn't established. So a row that lingers here is a FAIL, and its cause is found in the running app (the resources tick's log lines, timed) before it's fixed.

Each row's sheet is its tracked `docs/qa/0.4-walks/<ID>.md`, made by `scenario.py sheet` (`lab_scenario.sh setup` never writes one). A row that fails is fixed, the build is made again, and that row and every row it touches are walked again. The table keeps both results.

## 4. The persona runs (AJ-E3307)

The protocol is `docs/qa/persona-validation.md`: Builder, Tester and Critic in separate contexts; three fresh runs per flow; the expectation written before each step; scores with a one-line reason. The personas and their flows are in `docs/qa/0.4-walks/personas/`.

Tester runs: **sonnet**, a fresh context per run. Critic, triage and the report's ranking: **Opus**, on the findings only. Coverage counts and the score grid's formatting: **haiku**.

| Persona | Flows | Where | Runs |
|---|---|---|---|
| Maya (×2 weight) | M1 First run · M2 What needs me · M3 Room left · M4 Out mid-feature · M5 Back after reset | M1 in the RC's native window on a lab with fixture folders (by pid); M2 with `scripts/qa/lab_ask.py`; M3–M5 lab | 3 each (M3 run 1, the pilot, is done) |
| Devon | D1 First run · D2 Morning triage · D3 Find one of fifty · D4 The laptop is struggling · D5 Start a batch | D1 native window; D3 replica; D2, D4, D5 lab (D4's load bounded, with a trap, and reaped after) | 3 each |
| Priya | R1 First run · R2 Right account for the client · R3 Who touched this repo · R4 Keep a client apart | R1 native window; R2–R4 lab | 3 each |

Then the heuristic sweep (a fresh agent, screenshots only), the Critic, triage, and the maintainer's calibration runs. The report goes to the maintainer in the form the protocol's last section sets out.

What no agent can walk, and the maintainer's calibration covers: the installer, macOS permission prompts, the menu bar, system notifications.

## 5. The maintainer's steps in this run

- Any macOS prompt in a 7b row (Automation, notifications): the maintainer clicks.
- The calibration runs: one flow per persona, by the maintainer.
- Not in this run, already deferred or waiting: Codex→Codex cells (D-91, a second Codex account); the second Mac (D-89, after the release); AJ-E808's Claude→Claude file history (a second Claude profile sanctioned for switching).
- AJ-E1907's live start of a task's session: there's no task-start command on the lab write list. It needs either the maintainer's approval to add one (lab-only, with refusal tests), or a sitting.

## 6. The summary (filled in at the end)

What passed, what was fixed (with its commit), what's known and why, and what waits on a decision.
