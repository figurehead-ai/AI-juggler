# 0.4 release candidate: the plan, then the record (AJ-E3306, AJ-E3307)

The release candidate is walked whole, in one run, on the built app, before 0.4 ships (`docs/qa/0.4-validation-plan.md` §7–§8). This page is the run's plan first. Each row's result is filled in as it runs, and the summary for the maintainer ends it.

## 0.4.6 (the Stable candidate): this run's shape

- **The budget** (the coordinating session's call, 2026-11-16, because the maintainer's quota ran out several times that day): the whole scenario table in §3 once; the personas in §4 at **one run per flow**, and **three runs** for AJ-E3307 R7 ("see it work") and for any flow that scores under 4. A run that shows instability gets its re-runs added. About 6M tokens in all, against about 9–10M for three runs of every flow.
- **The build:** a Dev-identity release build of release/0.4.6 (`scripts/build_dev_app.sh`), never a production identity beside the installed app.
- **The sign-ins** (D-122: one sitting, one page, before 0.4.6 ships, the Dev identity handed between the 0.5 and 0.4.6 sessions): the lab Codex for AJ-E3013 C1/C2; for E3002-R3, OpenCode through 0.5's T16 lab A used in place (0.4.6 gives OpenCode its tools per launch) and Grok Build in a home 0.4.6 makes, with its own sign-in (0.4.6 never edits a Grok home it didn't make, so a home used in place has no agent tools).
- Rows already proven on 0.4.6 since the last RC are walked again here all the same (E3306 R1: every scenario, one run).

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
| Release scope | `python3 scripts/release_scope.py report 0.4`: every row proven or deferred by a D-number | **0.4.6:** `check 0.4.6 --channel stable` on rc3h: 141 of 146 done and proven; open AJ-E3002, E3013, E3304, E3306, E3307. AJ-E3002 is done since (142 of 146). The four left gate Stable, not beta (main, 2026-11-19) |
| Preflight | `just preflight origin/main` (the fast local check, which stamps the commit for the push); the heavy gates run on GitHub through the release PR | **0.4.6:** not run yet: it runs once, before the push. On rc3h's code (e97b03114): static green; `just verify-tests` 4 080 of 4 080, with the types, doctests, bridge, leaked-fixture and Floor checks; the visual gate 331 of 343, and the other 12 (7 crashed browsers, 5 timeouts) passed on a re-run, no baseline moved |
| Re-baseline | the one visual re-baseline (notes: the side-session notes' `rebaseline-0.4.1-notes.md`), every capture opened by eye, the critique stamped after staging | **0.4.6:** rc3 in a81fdacca and 6c2f07f70 (343 pass; 465 changed baselines, each explained; a Sonnet review, its flags judged by Opus); the pool galleries' critiques stamped in 42f3a3d64; rc3g's moved baselines in f9167f4fe, each checked by eye |
| AJ-E3305 | the 24 families re-scored on that gallery (`docs/qa/0.4-walks/design.md`); lows fixed or sent to the maintainer | **0.4.6:** not re-scored as a whole. The redesigned screens (the pool wizard, the setup canvas, the running canvas; D-126) were put side by side with the approved mock-ups in both themes over three rounds, fixed (49f802c69, dfb1eab7a) and approved by main |
| The installed app | quit while a row needs the single-instance socket (`tests/second_launch.rs` hands over to it otherwise) | **0.4.6:** not needed: each RC was a Dev-identity build, started through `scripts/qa/lab_env.sh` and checked by `ps`, and the only Dev instance running (the one before it quit first) |
| Desktop-member host checks (AJ-E3013) | `just desktop-host-checks`, with the Claude app running and a Code-tab session open: read-only, it starts, locks and changes nothing. Every running Claude app's main process names its profile the one strict way and holds that profile's lock; the running app, its `disclaimer` and its Claude Code meet the pinned requirements; the app's fuses stay off. A FAIL after a Claude app update means attestation now refuses ("profile lock held by another process, maybe after a Claude app update"): no connector row passes, tell the maintainer before the release | **0.4.6:** no run recorded on rc3a–rc3h |

## 3. Every scenario again (AJ-E3306 req. 1)

Where each runs:
- **replica:** the replica HOME (`just replica`), the RC run with `HOME` set to it.
- **lab:** a scenario lab (`scripts/qa/lab_scenario.sh setup ID`), the RC run with `HOME` set to the lab, through the lab write mode where the row needs writes.
- **live-codex:** the real Codex sign-in where it is (ADR-0027 condition 7).
- **live-claude:** Claude Code in the Dev window on the real HOME, driven by pid, harmless commands in `~/lab/` (ADR-0027 condition 7b).
- **mock:** the design preview.

Every row below is driven by **sonnet** from its sheet. **Opus** checks each FAIL, and each PASS that closes a requirement, before it's recorded.

**Results:** 0.4.6's candidates rc1 to rc3h (2026-11-16..19), from the walk sheets linked in each row and the release session's Opus-checked notes. Rows without a sheet link have their results in those notes only.

| ID | Scenario | Where | Steps | Who | Result |
|---|---|---|---|---|---|
| A1 | Open a Claude app profile | replica | 1 | agent | PASS on rc1; a folder path broken mid-word fixed (07021cf60), PASS on rc2 |
| A2 | Everything on one subscription | replica | 1 | agent | PASS on rc1 |
| A3 | From a session to its App and Account | replica | 2 | agent | PASS on rc1 |
| A4 | Projects an App was used for | replica | 1 | agent | PASS on rc1 |
| A5 | What an out-of-room Account affects | replica | 1 | agent | PASS on rc1 |
| A6 | Three Claude app profiles on two subscriptions | replica | 3 | agent | PASS on rc1 |
| A7 | Start Claude Code on the personal plan | replica, lab | 2 | agent | PASS on rc1 |
| A8 | Rename the work subscription | replica | 2 | agent | PASS on rc1 with four findings (§6), fixed, PASS on rc2. ⌘K not walked (the drive tool has no modifier keys) |
| A9 | Upgrade: one Account per sign-in | lab | 2 | agent | PASS on rc1; the cut-off Account title fixed (65a4a4029), PASS on rc2 |
| A10 | A sign-in can't be read | lab | 1 | agent | PASS on rc1 |
| K1 | The chain | replica | 5 | agent | PASS on rc1; an Account's app count fixed (5fee0a227), PASS on rc2 |
| K2 | Every page, every tab | replica | 6 | agent | FAIL on rc1 (counts, a tab's highlight, History's menus, a blank terminal, "Show all"); fixed (§6), PASS on rc2. Steps 3 and 6 not walked: they need live launches |
| K3 | Notices land | lab | 2 | agent; the system notification itself is the maintainer's | PASS on rc1 and rc2. Not walked: the sign-in and "may be stuck" notices (the lab raises neither); the macOS notification is the maintainer's |
| K4 | Pickers | replica | 2 | agent | Step 1 PASS on rc1, step 2 partly (the App picker's names, §6); PASS on rc2. Not walked: the keyboard-only part, the Tasks picker, the related lists' pickers |
| N1 | The usual | replica, lab | 2 | agent | PASS on rc1 (the start checked by `ps`); ⌘↩ not walked |
| N2 | Change it | replica | 1 | agent | PASS on rc1; its App picker PASS on rc2 |
| N3 | The rare options | replica | 2 | agent | PASS on rc1 |
| N4 | First time | replica | 1 | agent | PASS on rc1 |
| N5 | The usual is out of room | replica | 1 | agent | PASS on rc1 |
| P1 | Writer and reviewer | lab, live-claude, live-codex | 9 | agent under 7b; the maintainer for any prompt | Live on rc3g ([P1](P1.md)): steps 1–3, 7 and 8 PASS; 9 PARTIAL (the connector isn't added to aij-test, which holds real sessions); 4–5 not walked (the Lab account out of room, and Dev refuses the real Codex folder); 6 not walked (needs an account that runs out live) |
| P2 | Feature team | lab, live-claude | 6 | agent under 7b | Live on rc3f: 6 of 6 PASS ([P2](P2.md)); its findings fixed in rc3g. On the rc3a/rc3b lab: the lead's ties drawn (cf78b1a18, eb6206c8c) |
| P3 | Across repositories | lab | 4 | agent | PARTIAL on rc1: step 1 PASS; 2–3 not walked (they need an open docs session); 4 looked at only |
| P4 | A reviewer for everything | lab, live-codex | 4 | agent | Step 2 PASS on the rc3b lab; live on rc3g: steps 1, 3 and 4 PASS ([P4](P4.md)); its findings fixed in rc3h |
| P5 | Rules by account | lab | 4 | agent | PARTIAL on rc1: steps 1–2 PASS; 3 not walked (canvas nodes took no press; every node is a button since D-126, not walked again); 4–5 not walked (they need a sign-in changed, not allowed in labs) |
| P6 | A build Mac | mock | 2 | agent (pools across Macs are 0.5) | PASS on rc1 (the lab part); the mock part not walked |
| P7 | Sealed client project | lab (step 3 live-claude) | 5 | agent | Step 3 live on rc3f: PASS, Claude's own message into the sealed session refused ([P7](P7.md)). The other steps need live agents: looked at on the rc3a lab only, four small follow-ups (§6) |
| P8 | Scouts | lab | 3 | agent | PARTIAL on rc1: step 3 and step 4's label PASS; two sign-in rows alike fixed (38c281d57); FAIL on rc2, fixed (9c75074b0), not walked again. Steps 1–2 not walked (sign-ins); step 4's shell needs a live Kimi Code |
| P9 | Moving from 0.3 | lab | 3 | agent | rc1: steps 2–3 PASS; step 1 partly ("Go back to an earlier version" didn't open on an accessibility press); every disclosure moved to one that does (bc9e107dc to a7b0b35ae), not walked again |
| P10 | Set up by hand, first time | mock, lab | 3 | agent | PASS on rc1 (the lab part) |
| P11 | Overlap and deletion | lab | 3 | agent | Steps 2–3 PASS on rc1; step 1 live: waking PASS on rc3g ([P11](P11.md)), the hop pause PASS on rc3h (paused at the limit of 2, asked in Needs you) |
| PF1 | Persona first run | lab (native window) | 1 | agent, by pid | Not walked on 0.4.6 |
| PM3 | Persona Maya: which account has room | lab | 1 | agent | PASS on rc1; Start not walked (it would start a real session) |
| R1 | The runtime lab | lab, live-codex | 6 | agent (Claude steps 3–4 under 7b) | Not walked on 0.4.6: it needs the lab Codex sign-in (the maintainer's, with AJ-E3013 R1) |
| X1 | A day | lab, rc, live-claude, live-codex | 6 | agent; the crash and the restart on the RC | Step 1 live PASS on rc3f (clarity 3: two presses, D-119); 3, 4, 5b PASS on rc1 (the crash: relaunched within 6 s, everything in place); 5a FAIL on rc3a, fixed, PASS on rc3b ([X1](X1.md)). Not walked: 2 (needs an account that runs out live), 6 (needs the maintainer's sign-in to the vendor's usage page) |
| X2 | Load: a 50-member pool, 1 000+ ended sessions | lab, rc | 2 | agent | PASS on rc1: a pool opens in 372–599 ms by the drive tool (390–462 warm), its content there at once; CPU peak 20 % after a restart, then under 3 %; the lab has 50 sessions (the 1 000-session scale is K1's and K2's); the join log fixed (e7c9dd5b9) |
| NY1 | Archive takes a row out of Needs you at once | lab, rc (step 3 live-claude) | 3 | agent (step 3 under 7b) | Steps 1–2 PASS on rc1; FAIL on rc3a (after Unarchive, the rail and the bell stayed empty), fixed, PASS on rc3b in two fresh labs. Step 3 tried live twice on rc3g: the Claude app never rates a harmless lab turn review-ready; held by its test ([NY1](NY1.md)) |
| E3002-R3 | OpenCode and Grok Build with the agent tools ([sheet](E3002-R3.md)) | live (two test accounts, signed in by the maintainer), rc | 12 | agent under 7b; the maintainer for the two sign-ins | No walk on 0.4.6: it needs the D-122 sign-in sitting. The 0.4.1 live result stands (`docs/qa/pools-lab.md` §3; AJ-E3002 DONE) |

NY1, from the maintainer's report that a "stopped with an error" row stayed in Needs you until a restart:
1. Archive a session that stopped with an error. Time its row leaving Needs you and the bell count dropping. It must be at once, and at most one 5 s tick.
2. Unarchive it: the row comes back.
3. Archive a conversation in AI Juggler (another app's, finished, not looked at): its row leaves at once.

The code paths are fixed and tested (c81d79cd7, 8b36cf64f), but the cause of "until a restart" isn't established. So a row that lingers here is a FAIL, and its cause is found in the running app (the resources tick's log lines, timed) before it's fixed.

Each row's sheet is its tracked `docs/qa/0.4-walks/<ID>.md`, made by `scenario.py sheet` (`lab_scenario.sh setup` never writes one). A row that fails is fixed, the build is made again, and that row and every row it touches are walked again. The table keeps both results.

## 4. The persona runs (AJ-E3307)

The protocol is `docs/qa/persona-validation.md`: Builder, Tester and Critic in separate contexts; three fresh runs per flow; the expectation written before each step; scores with a one-line reason. The personas and their flows are in `docs/qa/0.4-walks/personas/`.

Tester runs: **sonnet**, a fresh context per run. Critic, triage and the report's ranking: **Opus**, on the findings only. Coverage counts and the score grid's formatting: **haiku**.

| Persona | Flows | Where | Runs | Result (0.4.6) |
|---|---|---|---|---|
| Maya (×2 weight) | M1 First run · M2 What needs me · M3 Room left · M4 Out mid-feature · M5 Back after reset | M1 in the RC's native window on a lab with fixture folders (by pid); M2 with `scripts/qa/lab_ask.py`; M3–M5 lab | 3 each (M3 run 1, the pilot, is done) | Not run on 0.4.6: Maya's flows go to the beta soak (main, 2026-11-19) |
| Devon | D1 First run · D2 Morning triage · D3 Find one of fifty · D4 The laptop is struggling · D5 Start a batch | D1 native window; D3 replica; D2, D4, D5 lab (D4's load bounded, with a trap, and reaped after) | 3 each | Not run on 0.4.6: Devon's flows go to the beta soak |
| Priya | R1 First run · R2 Right account for the client · R3 Who touched this repo · R4 Keep a client apart | R1 native window; R2–R4 lab | 3 each | R4 only, seven runs ([runs-0.4.6](personas/runs-0.4.6.md)): rc3a partly, partly, not (no run found Keep it apart); rc3f three times partly (all found it, two sealed); rc3g partly (ease 5, value 5; the new sheet read in one step). "Partly" is account binding, deferred to 0.6 or later (D-134). R1–R3 not run on 0.4.6 |

Then the heuristic sweep (a fresh agent, screenshots only), the Critic, triage, and the maintainer's calibration runs. The report goes to the maintainer in the form the protocol's last section sets out.

What no agent can walk, and the maintainer's calibration covers: the installer, macOS permission prompts, the menu bar, system notifications.

**0.4.6, the rest:** every R4 run passed its firewall check, and Opus was the Critic. Not run on 0.4.6: AJ-E3307 R7 ("see it work"), the heuristic sweep, and the maintainer's calibration runs.

## 5. The maintainer's steps in this run

- Any macOS prompt in a 7b row (Automation, notifications): the maintainer clicks.
- The calibration runs: one flow per persona, by the maintainer.
- Not in this run, already deferred or waiting: Codex→Codex cells (D-91, a second Codex account); the second Mac (D-89, after the release); AJ-E808's Claude→Claude file history (a second Claude profile sanctioned for switching).
- AJ-E1907's live start of a task's session: there's no task-start command on the lab write list. It needs either the maintainer's approval to add one (lab-only, with refusal tests), or a sitting.

## 7. On beta: the soak (D-114)

- **0.4.6 on beta, 2026-11-20.** Release PR figurehead-ai/AI-juggler#237 merged as 130fd935a; the Nightly built 0.4.6 from it (run 44732100274); Promote to beta, run 44736649566, approved by the maintainer and succeeded. Its sign-off record job is skipped for beta by design: it runs only for Stable, whose approval is the release's sign-off (D-109).
- **The beta feed** (`beta-channel` › `latest.json`) serves 0.4.6 for both Mac architectures (pub_date 2026-11-20T12:30:59Z), and the update archive's signature verifies against the app's public key, bound to version 0.4.6 (`scripts/updater_sig_check.py`).
- **The soak:** 7 days on beta from 2026-11-20, so Stable is due **2026-11-27 at the earliest** (D-114). Its walks, on the beta build: AJ-E3306 R1/R2 "in one run", AJ-E3304 P1 steps 4–5 (aij-test's CLI), AJ-E3307 Maya and Devon and the R6 report, and P1 step 9's first half on a throwaway lab. The "ran out" cells (P1 step 6, X1 step 2) stay open while their ruling is with the maintainer.

## 6. The summary (0.4.6, written 2026-11-19)

What passed, what was fixed (with its commit), what's known and why, and what waits on a decision.

**In short.** 0.4.6 was walked on its release candidates rc1 to rc3h, from 2026-11-16 to 11-19, in labs and live on the real Mac. Most bugs found were fixed and walked again on a later candidate (the list below says where not). The two possible blockers from the last walk were settled in one batch, built once into rc3h and walked again (below). The rest are under "Known": 17 smaller items that move to 0.5; and three limits already decided. It wasn't one run: the rows are spread over the candidates, with fixes between them. The walk of everything in one run happens in the beta soak (AJ-E3306 R1 and R2).

**The candidates.**
- rc1 4fc89b72a and rc2 8365e2d21: the lab rows, and rc2's first live checks.
- rc3a 6c2f07f70: the redesigned pool wizard and canvas (D-126), and the switch sheet that hides what can't be used (D-127, D-128).
- rc3b 849577e1b, rc3c eb6206c8c, rc3d 918242d9f, rc3e 653ea41fe, rc3f e32d3de55, rc3g f9167f4fe, and rc3h e97b03114, the last.
- Each was a Dev-identity release build, started through `scripts/qa/lab_env.sh` and checked by `ps`. A Sonnet driver walked, and Opus checked each verdict on its captures.

### What passed live

On the real Mac, with the Lab account and the aij-test profile (ADR-0027 7b), harmless commands in `~/lab/`:
- **What's open:** the presence check passed on rc2, rc3c, rc3d and rc3h. On rc3h, Open now shows exactly the 8 running Apps (`docs/qa/evidence/presence/0.4.6.txt`). A conversation resumed in a terminal, then opened again in its Claude app, shows as open in that app (rc3c).
- **Pools:**
  - P2's feature team, 6 of 6 (rc3f): held messages, Allow once, Deny answered in words, a line's own rule, a task handed out after your OK, and all of it back after a quit and reopen.
  - P7: Claude's own message into a sealed session refused (rc3f).
  - P4: a blocked message refused in words, a doc shared to the pool named, a pool the session isn't in refused by name (rc3g).
  - P11: two pools wake one shared session, one turn after the other (rc3g); a chain of replies pauses at its pool's limit of 2 and asks you (rc3h).
  - P1 steps 1–3, 7 and 8 (rc3g), and X1's morning: a held message approved from Needs you and delivered (rc3f).
- **Switching one Claude to another** (Copy, Move and Undo), between the aij-test profile and the Lab account: every check green on Claude Code 2.1.322, and the copy answered on the other side, 3 of 3 (a1fa3f0c3, 81e3c45c8). Move in place proven again on the Claude app 5.x, byte for byte both ways (`docs/qa/evidence/AJ-E212/claude-app-5x-2026-11-18.md`).
- **rc3g's fixes, walked again.** On rc3g: the App line in New session, quit and reopen before a first turn, Open in terminal before a first turn, "Deny and stop" in words, and the bell over Tasks. On rc3h: usage from a run in AI Juggler, a session's own name through Open in terminal and Stop, the pool names, and the message names, quotes and threads.
- **rc3h's walk:** 14 PASS, 3 PARTIAL, 3 FAIL and 1 not walked; the FAILs and PARTIALs are below.

### What passed in labs

- On throwaway lab homes: A1–A10, K1–K4, N1–N5, P6, P10, PM3 and X2 (rc1, with re-walks on rc2); P3 step 1, P5 steps 1–2, P8 step 3, P9 steps 2–3 and P11 steps 2–3 (rc1); X1 steps 3, 4 and 5 (rc1 to rc3b; after a forced crash AI Juggler came back within 6 s with everything in place); NY1 steps 1–2 (rc1 to rc3b). §3 names each part not walked.
- The gates on rc3h's code: the static checks; 4 080 of 4 080 tests, with the type, bridge and Floor checks; the visual gate with no baseline moved; and the accessibility scan of 884 screen states with no finding. Its menus check failed on one rc3g change (dd79e2861), which is reverted (4aaecfc00); then 10 of 10.
- Priya keeping a client apart (R4): from no run finding "Keep it apart" on rc3a, to all three finding it on rc3f, to one sheet read in one step on rc3g (ease 5, value 5). It stays "partly": she wants a repo tied to its client's account, which D-134 defers to 0.6 or later.

### What was fixed

Each was found on a candidate, fixed, and walked again on a later one, unless the line says otherwise.

On rc1 and rc2 (labs):
- New session, the switch list and the App picker named a terminal start after a desktop profile ("Claude Code CLI · Work Max"): f238e6927, 3b2d474a6, c88930d7b, 8cd9037b5.
- "Find this app's other profiles" broke a folder path mid-word: 07021cf60.
- Rename didn't select the old name when it opened: 3ddd16810.
- An App page scrolled sideways: d2821fedc.
- A page tab stayed lit for a moment after you left it: 0407f5105.
- A masked Account title was cut off with room left: 65a4a4029.
- An Account's "Its apps" said 67 sessions where there were 333: 5fee0a227.
- A pool's line counted ended sessions as open ("500 sessions · 50 finished · 500 ended"): 908653166.
- History's App and Tool menus didn't narrow each other: 5703cbbef.
- An ended session's terminal was blank for about a second: efc92a1a2.
- "Show all 200" and "Mark all 200 reviewed" counted only the sessions loaded: f92106332, bdc38a924.
- Usage said "Sessions 0" with 1 000 sessions: af8dffded.
- Two sign-in rows in Needs you read the same: 38c281d57, then 9c75074b0 (not walked again).
- "No lead for now" while the card said Lead: 03a4f8b54.
- A pool's log said "role=-" for each join: e7c9dd5b9 (held by its test, not walked again).
- "Go back to an earlier version" didn't open for an accessibility press: all 21 such folds replaced, bc9e107dc to a7b0b35ae and 92500d097 (not walked again).
- A conversation resumed in a terminal, then opened in its app, still showed as an ended run: b1d400f08, 73a286c8b. "Open in Claude app" now asks again until the app shows it: 73f625c04.
- A project's age read "20773 d ago": 90f8f8fa4 (not walked again).
- A rail row's state word ran over its time: 02af01fc5, 31bef9bb4 (held by the layout check, not walked again).
- A press could go through the New session sheet to the page under it: cc7a44bda, with 336c9f5b6 and 1556778f0 for the focus it moved (held by the gates, not walked again).

On rc3a and rc3b (labs):
- The running canvas drew no ties from the lead to its team: cf78b1a18, ec9089f95, eb6206c8c.
- After Unarchive, Needs you and the bell stayed empty until a restart: 6ca30cdcf, 82bbe8d4c, 849577e1b.
- A session picked up after a restart stayed Working after it ended: 7c42884f2, a376e6544.
- Priya couldn't find "Keep it apart", and the project's Agents card read the other way: e32d3de55 (main's rulings 1 and 2), then one plain sheet: c35c088d7, 537fcaa14, 930905a33.

On rc3c to rc3e (live):
- At start, Home showed two-day-old usage readings as current, with no age: 407a1cd8e, 70260a0d8, 807710f81 (in rc3g).
- A stopped run had no "Open in Claude app": 20147ea4a.
- The default Claude app wore another Account's Out mark: acadb1b67.
- "Starting Claude app…" stayed for over 80 s (4a2e91e19); then a start that worked wore a warning (653ea41fe).
- The presence check counted Codex sessions by an old rule (the check was wrong, not the app): 0791f7bee.
- Tests the full run caught: 0fa0753db, efb3b6619, 5a3268e62 (tests only).

On rc3f (live):
- Picking a folder in New session changed the App without saying so: 0d62207a0.
- The bell's panel opened under the pool's Tasks panel: 857695869.
- On reopen, two sessions were handed to terminals that failed at once: d4c840a46, 99f4d50d2.
- "Messages this week: None yet" beside three messages in Live: 4de23ca0a.
- A raw "error_during_execution" after "Deny and stop": 2e9887601.
- An ended Lead was named by its raw id on the Pools card: 85d5a674b, 764721772.

On rc3g (live):
- A run's limits stayed on its own panel, so the Runway said "100% left" beside "36% used": 2920e72e3, e06bc7f3e, 6320db395, 1f544196a.
- AI Juggler's own sessions showed as running in the app whose shell had started AI Juggler: fc0bd9684, 6666e4431 (not walked across a restart: below).
- Open in terminal dropped the name you gave a session: 8692f0f92, 57618fadb.
- A stopped session's title flickered to its first prompt: 647dca033, cfc08c234.
- "App changed … Undo" stayed after you chose the App yourself: 898fc7e73.
- A project's own pool had a card name the agents didn't accept: 2d8a9205e, 8c7ea5070 (one case left: below).
- "No one has joined yet" for about 18 s beside "3 open": 191349206.
- A Claude Code run in AI Juggler didn't show what you sent: fedf1ce4b, ec6577362, b867f1674.
- A thread's count of replies stopped at 6, never the pool's own limit: 58d4dae81 (its words: below).
- Writing to a member by its name failed: 11d8487a5, 5ff994fed.
- A reply's Live line quoted the first message: d26d29247.
- A session's Messages tab kept another session's thread: 890550ea8.
- The Pools cards reordered under the pointer: 7695e15e0, 2c062519a (they still move when a pool is made: below).
- "Model: <synthetic>" after a limit hit: 89115711d (not walked again).

Not a bug: "the bell can't close its own panel" came from the walk tool's way of pressing. Its change (dd79e2861) broke the menus' accessibility rule and is reverted (4aaecfc00), after rc3h was built.

### Known, and why

**The two possible blockers from rc3h's walk, settled in one batch (D-137), rc3h rebuilt at 8d1e9516b and walked again.**
- A Claude Code run's page that stayed "no turn yet" after Send: **not a bug.** The walk's window was fully covered by other windows, so the page was hidden and, by design, paused its live updates (AJ-E805); everything on it froze together, and opening the page again showed the turn. Walkers keep the window visible or reopen the page. The "Show them" view it led to, which called a session AI Juggler runs "Started outside AI Juggler" with no Send box, is fixed (45ac2facb; tests fail on the old code): the link shows only once a conversation is written, and such a conversation opens on its session. On the rebuilt rc3h the mislabelled view can't be reached: every way in opens the session's own page.
- "Don't deliver" on a chain paused at its hop limit: **was a real bug, fixed** (82bcc4f8d). It now ends the chain at that message: walked live, Home's card goes, nothing waits on you, the thread reads "Stopped: you didn't deliver «k-a»'s message past this pool's hop limit", and the message "Not sent"; Resume opens the thread again and sends nothing. Resume while the chain is paused lets the paused message wait as usual (ba4fd61b6, by its test; the live step was refused by AI Juggler's duplicate-message guard, as the walk resent the same prompt).
- Folded into the same build: going back from 0.4.6 names "0.4.5", not "schema 52" (691a8b86b, with a test that every database version is listed).

**Cells left open, each with its reason:**
- X1 step 2 and P1 step 6: they need an account that runs out of room during the walk.
- X1 step 6: it needs your sign-in to the vendor's usage page.
- P1 steps 4–5: the Lab account was out of room, and AI Juggler Dev refuses your real Codex folder. They can be walked with aij-test's terminal Claude Code, in the soak.
- P1 step 9: adding AI Juggler's connector to aij-test would give your own sessions there the pool tools. Its answering half needs a signed-in lab Claude app profile.
- NY1 step 3: the Claude app never rates a harmless lab turn "review ready". The rule is held by its test (`review_ready_leaves_needs_you.rs`).
- rc3h's restart check: the walker's quit was refused. The fix it checks is held by `driven_place_after_restart.rs`.
- rc3h's Open now check is PARTIAL: it was read from the sidebar's list, not opened from its own row.
- R1 (the runtime lab) and AJ-E3013 R1: they need the lab Codex sign-in.
- E3002-R3: no walk on 0.4.6 (it needs the D-122 sign-in sitting). 0.4.1's live result stands, and AJ-E3002 is DONE.
- Needs you after a sign-in: on rc2's real Mac the rail, Home and the bell agreed (12, 12, 12); the check right after a sign-in waits for one.
- Not walked in the labs: K2 steps 3 and 6, P3 steps 2–4, P5 steps 3–5, P7's steps other than 3, P8 steps 1–2 and P8 after its rc2 fix, P9 step 1 after its fix, PF1, K4's keyboard-only part, ⌘K and ⌘↩. §3 says why for each.
- rc3a's P7 lab look left four small follow-ups, not checked since: a lone "No lead for now" on a canvas whose members a sealed pool holds; "Setup done" with the panel still in setup; "Who's in: No one yet" while the List shows a held member; the Pools card drawing 3 nodes for 2 sessions.
- The desktop host checks (AJ-E3013) and AJ-E3305's whole re-score weren't run on 0.4.6 (§2).

**Known limits, decided:**
- A folder signed in two ways (its Claude app on one account, its terminal on another) moves both its Apps between Accounts. It isn't new, and 0.4.6 has no safe stopgap. Fixed for 0.5 in AJ-E3210 (release/0.5, D-135).
- Switches through another service stay hidden, behind Settings › Switching › Beta; their live route is AJ-E818, in 0.5.
- A repo tied to its client's account (Priya's ask) is deferred to 0.6 or later (D-134).

**Moved to 0.5: 17 items.** None is a blocker by 0.4.6's rule (data loss, the wrong account or action, a crash, security or privacy, a core flow broken). rel-05 queued them as AJ-E3014 (1–7), AJ-E2311 (8–15) and AJ-E2025 (16–17), in 0c0959542 on release/0.5.
1. The Pools cards still move when a pool is made. A new pool should go at the end, and the order stay put.
2. A session only in a custom pool is refused when it names its project's own pool by the project's name ("web").
3. A paused chain's words read "3 … over this pool's limit of 2" where you expect "N of 2". A pool doesn't pause at its limit for a desktop member, or a terminal AI Juggler can't type into.
4. Pool setup's side panel says "Open a session in web: it joins" with the every-new-session switch off. Who's in repeats the connector paragraph, and the side panel doesn't say who joins now and who later.
5. Settings › Agent access doesn't shorten a told-apart pool name.
6. Deleting a pool is only at the foot of its settings. The List view's right panel doesn't scroll by wheel.
7. Approving a held message in a Work together pool takes two presses, Allow once and then Deliver (D-119; X1's clarity 3): a question for you.
8. During a hand-over to a terminal, the sidebar and Home list the session twice.
9. New session suggests other projects first, Recent reshuffles, and the layout shifts when its memory line appears.
10. The Stop toast names the folder, not the session.
11. The switch dialog at 1280×800 doesn't show "what comes along" or Cancel, and doesn't scroll there.
12. The pool wizard's steps can't be pressed in its left rail.
13. Recovery limits: a run whose conversation file was deleted still ends in a failed start; a session with no turn, quit under a build before d4c840a46, fails once after the upgrade; a switch back from another Mac takes the original's name; a rare race in the live look.
14. Whether Codex shows a message you add during its turn as yours isn't checked live.
15. Usage readings: a whole "1" from Claude would read as 100%, and a week on one model is labelled "Weekly".
16. Claude's turn-end codes move from the screen code into its adapter.
17. A session AI Juggler runs in an outside terminal is placed by its own process, before its conversation is read.

### What waits on you

- Your sign-ins: the lab Codex (AJ-E3013 R1, the runtime lab), the vendor's usage page (X1 step 6), a signed-in lab Claude app profile (P1 step 9), and the D-122 sitting for E3002-R3's two test accounts.
- Whether to defer the out-of-room cells (X1 step 2, P1 step 6).
- Your calibration runs: one flow per persona.
- Item 7 above: one press or two to approve a held message.
- Stable: AJ-E3013, E3304, E3306 and E3307 are still open, so 0.4 isn't complete for Stable (142 of 146, with AJ-E3002 done). Main's call, 2026-11-19: none of them holds beta.

### The run in one go

It happens in the beta soak: AJ-E3306 R1 and R2 (every scenario, X1 and X2, on one build), AJ-E3304's P1 steps 4–5 with aij-test's terminal Claude Code, and AJ-E3307's Maya and Devon flows with its report (R6).
