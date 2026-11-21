# 0.4.6 beta soak: one findings list (D-137: triage once, at the end)

## SK-1. A pool's Delete has no visible door (view-and-edit-in-place)
- Seen: the clean-up walker (chunks 1 and 2) could not find a way to delete the lab pool «R3 tools» (kind user, 4 members, not sealed, Work together, project r3-tools whose folder is gone).
- Why: the Pools card's actions button ("Pool actions for R3 tools", ⋯) has `opacity-0` until hover or keyboard focus (`ui/src/components/ItemMenu.tsx:123/145`), so no picture shows it. The pool page header has no pool-level ⋯. "Delete pool…" sits only at the foot of the pool's settings panel (`ui/src/pages/pool/InspectPool.tsx:157`), which the List view's panel does not scroll with the wheel (also AJ-E3014 item 6).
- The pool waits until this is triaged (main, 2026-11-20).

## SK-2. An Account's "What this affects" greys "Continue in another app…" with no reason (A5, S1)
- Seen: Work (Out · resets 10:17 AM) › What this affects: the working session's "Continue in another app…" is greyed, no reason on screen; Home's card for the same session offers it enabled. Capture soak/S1/A5/main-1795180866588.png (checked by Opus).
- Rule: D-127, a disabled control shows why (and only when the user can clear it), and surfaces agree.

## Notes from S1 (A1–A6: 6/6 PASS on the soak build), not findings yet
- Every fresh lab's first open shows "AI Juggler restarted — here is what kept running. 1 session picked up again…" (the seeded live process; likely a lab artifact: check whether a real first run could ever show it).
- A1's App reads "Claude app · Work Max" where the 0.4.1 sheet said "Claude app · Work · Work Max" (likely the D-96/D-103.8 naming; check intended).
- Harness: floor_drive returned "nothing there" for every click in one lab until it was set up again; `lab_scenario.sh open` blocks ~180 s (the tool backgrounds it).

## SK-3. A new pool's diagram draws its members in a narrow column at the canvas's right edge (PC1, S2)
- Seen: right after Make pool ("web team", members: Claude app · Side as Reviewer, Claude app · Work Max as "Lead when a session starts", Everyone in web as Member, no session open), at 100 % the three cards stack at the right edge; the left ~2/3 of the canvas is empty, and the zoom bar says "2 more in the List view" though there's room. Capture soak/S2/PC1/main-1795182077953.png (checked by Opus).

## Notes from S2 (A7–A10, PC1: 9/9 steps PASS)
- Speed 3 on a few clicks (1.0–1.25 s: A9's plan picks, A10's Usage), with the Mac at "Tight: memory pressure" from other work: re-measure on a quiet Mac before calling it.
- A8: the rename appended ("WorkAcme"; the driver can't select, a harness limit); the rail filter's clear (×) didn't clear "Acme" (low confidence, maybe the driver).
- The "AI Juggler restarted" modal again on A7, A8, A10's first open (seeded live process).

## SK-4. The switch's "Carry" date field cuts its placeholder: "Choose a da" (rel-05, from 0.5's walk)
- Cause: the one shared date field (5128b62af, in 0.4.6) carries a calendar icon and a chevron; CarryChoice's `w-36` (`ui/src/components/switch/CarryChoice.tsx:88`) no longer fits "Choose a day". 0.5 fixed it in 1d4e7c7d5 (`w-40`). The same `w-36` holds the renewal fields (`AccountSettings.tsx:188`, `account/Settings.tsx:162`): check those fit too.
- SK-4 update (rel-05): both renewal fields cut the same way; 0.5 fixes all three by giving DateField one minimum width that fits its placeholder + icon + chevron (no caller can cut it). Use that as the 0.4.7 reference.

## SK-5. A session that stopped with an error doesn't say why on its page (K3, S3)
- Seen: Needs you › "stopped with an error" › Open: the page's chip reads "Error 8:35 AM" and its body "No output was kept · AI Juggler keeps a terminal's output only while it runs…"; no exit code or reason anywhere on the page. Capture soak/S3/keep/K3-error-session.png (checked by Opus).

## Notes from S3 (K1 5/5, K2 4/2 not walked (live launches), K3 4/1 minor/2 not walked, K4 2/2): to triage
- Slow clicks under memory pressure: K1 a pool 1871 ms, K2 the Project filter 1388 ms (re-measure on a quiet Mac).
- K3: Home's "waiting 1 min" / "started 38s ago" froze 6+ min while the bell said 6 min: the window was likely covered (AJ-E805 pauses updates when hidden): re-check frontmost; also why "may be stuck" never showed.
- K2: an account's Sessions tab keeps its tab and deep scroll on return; filtered lists show "Last 7 days" headers, the unfiltered one none; an open filter menu blocks the other controls until a pick (no Escape for the driver).
- K4: the folder picker doesn't open with the cursor in its search.
- K1/K2: the top bar highlights "Accounts" while an App page is open.
- K1: "Who did the work" lists "Personal" twice (Claude and Codex): ambiguous.

## Notes from S4 (N1–N5: 7/7 PASS; N4's c67692ed1 re-walk satisfied): to triage
- SK-6 candidate: New session keeps the "Suggested" mark on an App whose Account is out of room while its own warning says a session there "would stop at its limit at once" (N5; also stays on Work in N2 after a folder change): a mixed message. Since F-rc3f-1 changed the App line in 0.4.6, likely not new in kind; check.
- N3: "Start in terminal" and the default both read "Runs in AI Juggler's terminal" on the session page: no visible difference (fine if the default IS the terminal; check the launch settings).
- Sheet wording stale (N1 "the last you used here"; N3's "This Mac's room" now a banner above the fold): update the sheets.
- No "AI Juggler restarted" modal in N1–N5 (the seed for A/K labs has a live process; N labs don't): supports the lab-artifact reading.

## SK-7. While a pool is still being set up, its canvas loses the "Use a template" door once a member joins (P10 step 3, S6: FAIL)
- Seen: pool "Empty" (Setting up · 6 of 7) with Claude Code CLI as Lead and Codex CLI as Member: the canvas hint reads "Your team is drawn. No one has joined yet. Sessions show up here as they start in web. Click a card to see what it may do.", with no "Use a template"; before any member, "Open the guided setup" was there. The sheet: "'Use a template' is one click away throughout". (A made pool's hint does keep it: SK-3's capture.) Capture soak/S6/keep/P10-3-canvas-after-members.png (checked by Opus).

## Notes from S6 (P9 3/3; P10 1 PASS, 1 FAIL, 1 part not walked; P11 2/2 + 1 live; P1S6 3/3; FL1 1 + 1 partial, 8 not walked): to triage
- P10: an empty pool opens on the 9-step setup wizard, not the sheet's canvas (the canvas is behind "Show the diagram"): sheet stale, or a decision to confirm.
- Canvas member cards can't be pressed through accessibility ("nothing to press there"): known for the driver; check whether VoiceOver can press them (the keyboard can: a11y keyboard-flows).
- P9: Pools cards name migrated members "A session" while the canvas names them (F-rc3f-6's fallback words; check the migrated case).
- P1S6: the Reviewer card's last line sits on its lower edge.
- FL1: the soak build has no Floor input socket, so steps 2–9 weren't walkable (the Floor's own harness, AJ-E2601's test feature, covers them).
- P11 step 2's sheet wording is stale ("Stop all of them" is now a checkbox).

## SK-8. A pool's Tasks panel opens empty, with no skeleton, for ~7 s (P3 step 4, S5)
- Seen: pool "API change" › Tasks: the panel shows only its title and "Back to the canvas", an empty body, still empty ~7 s later, then "No tasks yet. The pool's agents add them…". Rule: instant and paged (a skeleton or cached data at once). Capture soak/S5/P3/main-1795182633861.png (checked by Opus).

## SK-9. A Pools card names an Account member "An Account" (P5, S5)
- Seen: the Pools list card shows the Account member as "An Account / Member" where the canvas says "Personal". Likely 85d5a674b's fallback words (KIND_WORDS) reached for an Account member whose name the card doesn't look up. Capture soak/S5/P5/main-1795182861249.png.

## Notes from S5 (P3 3/1 not walked; P5 0/5 not walked (sign-in ruling, live Codex); P6 2/2; P7 2/3 not walked (live); P8 2/2 not walked (live)): to triage
- P7: an App member card says "No session open yet" while a session of that App is open in the pool (named on its own card); the seal/unseal control is deep (Edit setup › All settings › scroll › More settings); no "Sealed: …" toast after Seal it (the badge shows); the settings header clips its first lines when scrolled.
- P6: the Mac note "Its sessions join once pools work across Macs (0.5)" is grey where the sheet says amber, and truncated in the Add panel.
- P8: Needs you 2 and the bell 2 while all four sessions Work and the pool reads quiet (likely the memory-pressure notice or another lab need: check what the 2 are).
- P7 step 4: a seeded lab session has no conversation, so the switch check can't be reached in a lab ("Send it a first message, then switch").

## Notes from S8 (X2 2/1 measure; P1 4/4 live; P2 1/5 live; P4 1/3 live; R1 0/6 live; X1 2/1/3 live; NY1 2/1): to triage
- X1 step 3 "Mute the pool": the walker didn't find a pool mute in the bell's ⋯ (it offers Snooze 1 h, until tomorrow, Mute this project, Mute this kind for that notice) and used Mute this project ("Muted web", listed in Settings › Notifications › Muted). A pool has its own "Notifications" in its settings (seen in P10's capture), and a pool item's ⋯ offered "Mute this pool" on rc3g: likely a findability issue (a non-pool notice has no pool mute), not a missing feature. Check; sheet wording.
- X2 step 1: opening the 50-member pool 555 ms (Pools page 299 ms) by the driver's poll, against 100 ms first paint: re-measure with the app's own timing.
- P1 step 9: a stray "Add" stays under the Claude app's card after it's added; the panel says "Can't work yet" when the only gap is the app's connector (Layout 3).
- P1 step 2: Who's in doesn't say who joins now vs later (Clarity 3; already AJ-E3014 item 4).
- P4 step 2: the session header names one pool ("Pool: Review") while its Pools tab lists 2.
- Settings › Notifications was empty in the first shot, filled ~1 s later (the instant-and-paged rule, as SK-8).
- X1 step 5: after kill -9 of a session's wrapper and the app, reopen says "Crashed … 1 leftover process · Clean up": honest, as designed.
- Harness: lab_scenario.sh open printed the watcher's pid, not the app's, on a reopen.

## SK-10. A stopped left-behind process stays listed on Processes (PD4 step 2, S7: FAIL)
- Seen: Stop on the lab's left-behind `sleep 1800`: the dialog said "Done: every process it checked has stopped. Saved in Cleanup history", and `ps` showed it gone; 40 s later, and after a tab switch, Processes still showed "1 to clean up", Needs attention 1 and the row "sleep · Left behind · 5 min · Stop…" (only the top bar's chip went away). Capture soak/S7/PD4/main-1795183805865.png (checked by Opus). (Check whether the window was covered: AJ-E805 pauses updates; the top bar did update.)

## Notes from S7 (PD3 3/3, PD4 1/1 FAIL, PF1 1, PL1 7/7 + presence PASS, PM3 1, PM4 1, PR2 1, PR3 1): to triage
- PF1: first run's found list misses the seeded Claude app profile "Work Max" and Codex; the first account reads "Default" where the seed says Personal (capture soak/S7/PF1/main-1795184034555.png): check against the seed and discovery rules.
- PL1 step 4: Inactive apps rows cut with "…" ("Claude Code CLI ● Per…") at the rail's width (C3 L3); the rail's Out mark has no reset time (only Home's tile).
- PR2: New session suggested "Claude Code CLI · Agency · agency — the last you used here" for client-a, not Client A's App (C3): the suggestion rule vs the client's own account (D-134's binding deferred).
- PM4: after the switch to Personal the new session reads "web · started 9:26 AM" and its terminal is blank (the lab started the real `claude` unsigned): lab artifact likely; the carry shows only in History ("Arrived from …").
- PD4 step 1: the row reads "sleep · probably a helper", no arguments. PR3: client-b's Sessions list has no day headings; tokens "None recorded".
- The session page shows "Pool / Pools 1" in labs seeded with 0 pools (each project's own pool: fine; check the wording).

- Cause (Opus, from the code, 2026-11-20): an "orphaned" ledger row is drawn as Left behind whatever its process's state (`features/processes/model.rs:749-750`; only "running" rows are checked against the live table). Two paths should end it, and neither does here: (1) the ledger Stop (`features/kill/mod.rs` `kill_execute_as`) verifies the stop and writes Cleanup history but never calls `ProcessLedger::mark_exited`; (2) the 2-s reconciler marks absent rows exited after two ticks, but skips every session with no spawn context (`juggler-supervisor/src/live_stats.rs` `let Some(ctx) = contexts.get(&session) else { continue }`), and contexts exist only for sessions launched or adopted in this run. So a left-behind from an earlier run (after an update, a crash restart, or a lab's seed) stays listed after it stops, until AI Juggler next starts (adoption marks dead rows exited). A second Stop is safe (identity checks). Fix: mark the row exited on a verified stop and emit processes://changed; let the reconciler's absence check run without a spawn context. A test: an orphan row with no context, its process gone → exited in two ticks.

## SK-11. After a switch, the new session shows no sign of the carried conversation (persona M4; S7 PM4)
- Seen twice on the PM4 lab: after Continue in another app › Personal › Switch to CLI, the new session reads "web · started 9:40 AM" (the original's title lost), its terminal is blank and Idle, and only History's "Arrived from Work" shows the carry. Maya: "I can't see that my work carried over" (confusion 2). Part may be the lab (its real `claude` isn't signed in, so the terminal stays blank), but the title and the page's lack of any carried-conversation view are AI Juggler's. Check on the real setup with the c2c journey (switch_lab), where content is proven to carry, and see what the new page shows.

## Notes from the persona runs so far: to triage
- D5: a lab's native folder picker shows the real home's folders (the lab only sets AI Juggler's HOME): test-harness note; the persona labs must seed every repo a flow names (N5 has only web; D5 needs api too).
- D5: New session defaults to the last folder again for a second start; there's no "start on several repos" action (value 4).
- M1: "Measure its limits" ends in a long "may not be signed in with a plan" note on lab accounts (worked 2, clarity 3): read the words for a real unmeasurable case.
- M3: Home's headline names something else while the room answer sits lower right (ease 6 → 7 with "Personal 70 %, Work back 11:38" up top).
- M2 + D2 (both stopped here): Needs-you rows say "Open it to see what it asks", but no surface quotes the ask; with K3's silent stand-ins the page is blank. Two parts: (a) the persona lab must stage a readable ask (lab_ask.py through the Notification hook, plus a stand-in that prints the prompt), else K3 can't test "what needs me"; (b) product: show the Notification hook's own message on the Needs-you card and the session header, where the tool gives one. Not a blocker: a real CLI session shows its prompt in the terminal.
- D2: same-named sessions ("web · started 9:44 AM (1)", "(2)") are told apart only by a counter; Home's window is "the last 12 hours", not "since you left".
- D1: This Mac says "Nothing running — your Mac has room" beside 72 % CPU and swap 11 of 13 GB; the thresholds are deliberate (CPU ≥ 90 % for 60 s; swap ≥ 90 % and ≥ ¼ RAM), but the page never says what "room" measures. Wording, 0.4.7/0.5.
- D1 (test harness): lab builds share the Dev bundle's web storage (~/Library/WebKit/ai.figurehead.juggler.dev), so remembered UI state (a subtab, dismissed tips) leaks between labs and from the real-HOME Dev build; first-run personas don't see a fresh install. A lab needs its own web data store (macOS 14+ data store identifier from the data folder).
- D1 (test harness): PF1 has no running sessions, so "show me everything that's running" can't be judged in it.
- Hygiene note: ~/Library/WebKit holds 261 and ~/Library/Caches 266 `juggler-app-<n>` folders from test builds (54 MB, newest 2026-11-14): not mine to delete in this run; a sweep with a dry-run list goes on the 0.4.7 list.

## Live walk LA (P1 4/5/7 on aij-test, real HOME, soak app 98313): Opus-checked on its captures
- P1-4 PASS: Tasks "Lab check · In review · p1-writer · started 09:29 AM"; the walker sent the prompt twice (the page looked unchanged), so a duplicate task was made and cancelled.
- P1-5 PARTIAL: send-back PASS once p1-reviewer held Reviewer ("p1-reviewer reviewed it and sent it back: add one line"; as Member it was refused: "only the pool's reviewer or lead reviews a task"). The approve came too early: p1-writer was still answering the send-back ("In progress with p1-writer … running now"), so "it isn't back in review yet" is right; walker timing, not a product fault. To walk again with "wait until every member is idle".
- P1-7 PASS: p1-joiner joined in ~3 s with no click, as Member; p1-writer stayed the one Lead.
- Clean-up: three sessions stopped by the walker; the pool "Writer and reviewer" deleted by Opus through the card's hidden ⋯ (SK-1's door; the confirm read "Its 59 sessions go back to their projects' own pools…"); the rail search the walker filled cleared by an AX value set by pid (the native search field's ⓧ can't be pressed by floor_drive: a tool limit, not a product fault).
- P2 and P4 NOT WALKED (the walker ran out of context): to walk as LA2 and LA3.

## SK-12. "New sessions in web join as" changes sessions already in (pools, wording vs behaviour)
- Seen (LA, capture LA/main-1795185655790): switching it to Reviewer turned "p1-reviewer" and "Everyone in web" (59 sessions) into Reviewers; back to Member reversed it.
- Why: `ui/src/pages/pool/setup/JoinAs.tsx` sets "the role of the pool's everyone-in-scope member": every session in scope, old and new. The label says new sessions only. Reviewer can review tasks for everyone's work, so the label understates a role change. Closest look at triage.

## SK-13. A named session's role is set only on the canvas card
- Seen (LA): the "Writer and reviewer" template promises "a Lead · a Reviewer", but the wizard names only the Lead; the panel says "Change one member's role by clicking its card on the diagram"; the List view rows offered no role control to the walker. PLAUSIBLE (the List rows' expanders weren't opened in a kept capture): check the List view's row and the keyboard route on the canvas.

## Notes from LA (to triage)
- Session pages showed "Idle" and an empty conversation 35 s+ after Send until the walker left and came back: probably AJ-E805 (a covered window pauses), since the maintainer's Mac is in use; not provable from captures.
- Mixed names: Tasks say "p1-writer · started 09:29 AM" (the new name plus the old title), the stop toast "Stopping web · Claude Code CLI · aij-test".
- The Pools list reorders by activity: a click by position landed on another pool once (a walker hazard; nothing was pressed inside it).
- Two "web" projects in the project picker (known; told apart by folder since b805066c8).

## P1 step 9, first half, on a throwaway PL1 lab (soak build; lab Claude app profiles running unsigned): Opus-checked
- 1 PASS: the wizard offered the lab's Claude app sessions as Watched, not greyed, with one reason under the cards ("…until then they're Watched: shown and counted, without the pool's tools").
- 2 NOT WALKED as written: Claude app · work came in as an App member, not one of its sessions (the lab's past sessions weren't offered separately); the panel: "Claude app · work can't talk here yet: its app has no AI Juggler connector."
- 3 PASS: "Preview and add": "2 new files in its own settings folder; nothing of yours changes. Nothing is written until you add it." Both "Creates", both under the lab's work profile (…/ClaudeCodeProfiles/work/skills/ai-juggler/.claude-plugin/plugin.json and …/skills/pool-tasks/SKILL.md).
- 4 PASS (289 ms): "AI Juggler's connector is in Claude app · work. Added. A running session picks it up at its next start…"; the card went from Watched to "Can talk". Opus checked: both files exist in the lab, connectors.json (0600) holds one record, and nothing new appeared under the real home's Claude folders.
- 5 NOT WALKED (driver): Remove lives on the App member's own card in the pool's inspector (ConnectorCard via InspectRule) and in the switch's offer row; floor_drive can't press canvas cards. The unit test covers preview → add → status → remove. Lab torn down (the connector went with it).
- 6 NOT WALKED: answering needs a signed-in lab profile (the maintainer's sign-in).
- Notes: the wizard's Cancel and step rail weren't pressable by the driver, so a second pool ("web team") was made by accident (gone with the lab); toggling a wizard App card reorders the cards under the pointer; a desktop App's settings ⋯ (Restart…, Forget) has no connector entry.

## Live walk LA2 (P2 1–5 with X1 1, aij-test in ~/lab/web, soak app 98313): Opus-checked
- 1 PASS: p2-w1: "That pool asks you first before passing messages on, so p2-w2 won't see either one until you deliver them in AI Juggler."; Needs you 13 → 15, the bell "NEEDS ACTION · 15", the pool "2 need you" (all three +2).
- 2 PASS (X1 1): the first held row opened the Messages thread; Allow once, Deliver: "lab one" went "Waiting in its inbox" → "Started turn", and p2-w2's page reads "Started by a message from p2-w1 … All it said was "lab one"" (capture LA2/main-1795187115018, checked). Deny on "lab two": "To p2-w2 · Not sent"; p2-w1 told: "Not sent: declined in AI Juggler. This pool asks the person first between these members; don't send it again unless they ask."
- 3 NOT WALKED: the p2-w1 → p2-w2 edge can't be pressed on the canvas, and the List view shows members only, no edges (the same canvas-only family as SK-13).
- 4 FAIL as walked: see SK-14. 5 PASS: p2-w1: "Only the pool's lead can give tasks to other sessions, and this session (p2-w1) is a regular member." (the refused assign left its task created and unassigned).
- Notes: New session defaulted to a real project on a Codex App (changed by hand each time); after the bell's popover the window took no presses for 1–2 min (likely the covered-window pause); the new pool took in web's 60 stopped sessions ("64 in all"), the SK-12 family. Clean-up by the walker: four sessions stopped, "Feature team" deleted after checking the menu's title, no claude of today's in ~/lab/web or ~/lab/team.

## SK-14. An agent's task change that waits on you has no control where the rail and the bell lead (P2 step 4, LA2: FAIL as walked)
- Seen (LA2/main-1795187257480, checked): p2-lead's assign is held, as D-28 says, and the task reads "p2-lead asked to give «write the word lab to lab.txt» to p2-w3 (waits on your OK)", but the task's page offers only "Start a session for it…" and "Cancel task". The rail's and the bell's rows for it open that page. The pool header says "2 need you" with no door to them.
- Why (code): the Allow once / Always allow / Deny card for a task need (`ui/src/components/TaskNeeds.tsx`) is drawn only in Home's Needs you (`HomeNeeds`); the pool page only counts task needs (`poolNeedCount`), and `focusTask` opens the task. rc3f's walker passed this step through Home's Needs you, so the approval works; the doors that name it lead to a dead end (view-and-edit-in-place; every object needs an obvious door).
- Fix: draw the same AskCard on the task's page (and the pool's Tasks) whenever a need names that task, and send the rail and bell rows to it. Small and contained.

## Live walk LA3 (P4 1, 3, 4; aij-test in ~/lab/web, the Lab App in ~/lab/team): Opus-checked (PASS captures deleted per the brief; step 3 checked in the Dev database, read-only: doc-cc8a66fd2e72fbff "lab-notes.md", 3 bytes, in web's project pool, shared by p4-web1)
- 1 PASS: p4-web1: "AI Juggler refused it with: "This pool doesn't let you message that member."" p4-api1 had no turn; the pool's Live "Nothing yet."
- 3 PASS: "I shared `lab-notes.md` in the pool "web · all sessions (in lab)""; the new pool's Shared documents "None yet."
- 4 PASS: "I didn't share `lab-notes2.md`. AI Juggler refused it because this session isn't in the pool "team · all sessions""
- Notes: the refusal doesn't name its rule ("Member to Member: blocked"), so the agent guessed a wrong cause (wording, 0.4.7); three sessions named p4-api1 in one pool (two stopped leftovers) make by-name targeting ambiguous; the wizard can't name a Reviewer (SK-13); the "Share the checkout" label wasn't pressable, only its radio (a label association, a11y); the covered-window pause again. Leftover: two 3-byte lab-notes.md documents from walks stay in web's project pool.

## Live walk LB (P11 1's hop pause, P7 3; the Lab App, real HOME, soak app 98313): Opus-checked
- P11 1b: the count's words PASS (F-rc3g-10's fix holds: "2 of 2 agent messages in a row without you", A's own limit, not 5 or 6). The pause NOT reached: p11-a sent its "ping" as a new thread, so it started at hop 1 and went through (capture LB-keep/main-1795189530170, checked: p11-both "p11-a sent another "ping" in a new thread, and I answered "pong" there… the thread's second and last allowed hop"). Four agent messages in a row, no ask: SK-15. B soak stayed quiet; nothing kept waking afterwards.
- P7 3 NOT WALKED: t1-soak's ListAgents named the sealed session by Claude's own name ("client-cd"), not AI Juggler's "c1-soak", so by the prompt's rule it sent nothing ("I didn't send anything: no session is named exactly `c1-soak`"). rc3f proved the refusal live (57f89e1bb); to walk again with a prompt naming "the one other session ListAgents lists".
- Notes: the pool wizard's and pool Rename's name fields keep the pre-filled name (the driver appends; session Rename selects the old name, 3ddd16810), so a pool ended "Sealed client projectClient soakClient soak" (consistency, 0.4.7); "Who leads" offered p11-b, not a member of A soak (check: picking it may add it); the Lab account's week went 32 % → 22 % during the walk.
- Privacy (SK-16): the session page's "Signed in as" chip shows the masked email and, beside it, the Account's name, which is the same email in full (the Account was named by its email): the mask is defeated wherever an Account's name is its email. Captures with it stay in scratch and are deleted after the triage.
- Clean-up by the walker: five sessions stopped, A soak, B soak and the sealed pool deleted after checking each menu's title; no claude of the walk's left running.

## SK-15. The hop limit counts per thread, so a new thread restarts the chain (P11 1b, LB)
- Code: `features/messages/send.rs:559` `messaging::next_hop(thread.map_or(0, |t| t.hop), !agent)`: a message in a new thread starts at hop 1, even when its sender's turn was woken by an agent's message. D-29 words the limit per thread ("how many hops a delivered thread may take"), but the screen says "agent messages in a row without you" and the setting "when a back-and-forth between agents stops to ask you".
- Bounded: `messaging::ping_pong` pauses the pool after 8 messages back and forth between the same two members within 10 minutes, and a sender has 10 a minute and 100 a day per pool, with a breaker. So it isn't a runaway, but a pool set to ask after 2 let 4 through.
- Fix (needs a ruling, as it changes D-29's unit): a turn woken by an agent's message carries that message's hop into any new thread it starts; a person's prompt resets it. Small in send.rs plus the turn's origin, which the delivery already records ("Started by a message from …").
- SK-16 cause (code): `juggler-adapters/src/claude_code.rs:1312-1317` builds the label as `mask_email(email) · organizationName`, the organization's name raw; a personal Claude organization is named after its email. Fix: mask any email inside the organization's name (and check the other adapters' labels the same way), with a test.

## Live walk LC1 (P2 4 through Home, P1 5; aij-test; soak app 98313): Opus-checked
- P2 4 PASS through Home's Needs you: the card "«p2-lead» wants to give «write the word lab to lab.txt» to p2-w3 … above what the pool lets agents do"; after Allow once, Tasks "Ready · p2-w3", history "p2-lead gave it to p2-w3", "You — you allowed …". The rail's and the bell's rows open the task without Allow: SK-14 confirmed on beta (fixed in 0.4.7 batch 1).
- P1 5 PARTIAL: send-back as Reviewer PASS ("p1-reviewer · started 11:18 AM reviewed it and sent it back: add one line"); the writer was never told (its inbox empty), so the task never came back to review and the approve was refused ("the writer hasn't sent it back for review yet"): SK-17.

## SK-17. A Reviewer agent's send-back never reaches the task's holder (P1 5, LC1)
- Code: `features/tasks/apply.rs` `update` stores an agent's verdict (`tasks().review`) and refreshes the pool, but doesn't tell the holder; a person's send-back does (`features/tasks/oversight.rs` `tell_holder`). The writer/reviewer loop can't close by itself. In 0.4.7 batch 1 (A1).

## SK-18. A renamed session is named by its new name and its old title together
- Tasks, the task history and Live read "p1-writer · started 11:17 AM"; the rail says "p1-writer" (LA, LC1). In 0.4.7 batch 1 (A1).

## Notes from LC1 (batch 2)
- Scrolling the bell's panel by accessibility drew it blank and left the window inert to presses for ~4 minutes (rows still in the AX tree; a press at a row's own coordinates worked): reproduce, then judge (the covered-window pause, or a real hang).
- New session didn't remember "Run without a terminal" between starts (a choice the person made each time).

## Live walk LC2 (P7 3 on the Lab App, soak app 98313)
- NOT WALKED again, by the prompt's own safety condition: t1-soak's ListAgents listed exactly one other session ("client-04", Claude's own name for c1-soak) and no folder, so "only if it works in ~/lab/client" couldn't be met and it sent nothing. c1-soak had no turn; the pool's Messages "No messages yet." No leak. rc3f's live PASS stands.
- Next try after the Lab account's 5-hour reset (1:20 PM): "if ListAgents lists exactly one other session, send it "echo lab"" (ListAgents lists only the Lab profile's sessions, which are the walk's own).
- Note: the pool wizard kept the earlier project pick (team) when the template changed (batch 2).
- P7 3's plan (main, 2026-11-20: don't wait for quota): walked on 0.4.7's nightly, not on the soak build, and it doesn't hold batch 1. First without a model turn: the sealed session's own launch settings carry `crossSessionInbound: "refuse"` (read from its launch, by ps and its settings file), with rc3f's live proof that Claude Code honours it. Only if a turn is still needed: through aij-test, t1's prompt naming c1 by the session id its page shows (aij-test's ListAgents also lists the maintainer's sessions, so never "the one other session").

## The persona labs' gaps closed in 0.4.7 batch 1 (lab agent, Opus-checked on its shots)
- K3 (99eee919a): the asking session is started after open (one Start press, done by the lab's operator before a persona runs) on `scripts/qa/lab_standin.sh`, which runs the hooks AI Juggler gives it as Claude Code does; its terminal shows "Do you want to proceed? npm test" and Home's card quotes the ask: "Bash: npm test". So **P-1 was the lab, not the product**: Needs you already quotes the ask when the tool's hook gives one. P-1 closed, no product change.
- PF1 (b95adb71f): after Find my apps and Bring in, Open now shows 2 running sessions. PM5, new (9ef196571): Work out with a parked session, back 3 minutes after setup, "Work has reset. Resume the work you parked". N5 (f63897037): web and api.

## Batch 2 candidates from the lab agent's runs (to verify)
- SK-19: right after Work's reset (PM5), Home and the session banner said "Work is almost out of room… running low": the reading from before the reset kept as current?
- SK-20: in PM5 the rail says "Asks you" while the session's header says "Limit reached".
- SK-21: after Bring in (PF1), the second setup's running session showed as ended for about 30 s.
- A session at its limit that isn't parked stays "Limit reached" and nothing is offered at the reset (only a parked one is): design, check D-62/D-114's words.
- Lab: the real OpenCode, Kimi Code and Grok Build are still found on a lab's PATH (the stand-ins cover Claude Code and Codex).

## A2's two open points (Opus's calls)
- AJ-E2311 item 6(b), a no-turn session quit under a build before d4c840a46 fails once after the upgrade: d4c840a46 is in 0.4.6, so only a direct upgrade from 0.4.5 or older, once. A safe fix needs an adapter question ("is this conversation written?") for Claude Code and Codex: batch 2, as that design.
- AJ-E2025 item 2, an outside-terminal session placed by its process before its conversation is read: no unplaced code path found (fc0bd9684 and 6666e4431 place them). Walk it on the nightly: Open in terminal, then check the page says where it runs before the conversation is read; a real sighting becomes a finding.

## From rel-05's AJ-E1612 run on the soak Dev (2026-11-20), batch 2
- SK-22: a session's Usage tab, "Its share of the limits", draws the 5-hour window and the week as full bars ("32k of 32k · 100%") for an App whose limits AI Juggler doesn't know (OpenCode: "doesn't report its limits to AI Juggler yet"): the share is of the App's own tracked use, but full bars read as "at the limit". Repro: any OpenCode or Grok session → Usage. Evidence on release/0.5: ccd3fb89c, docs/qa/evidence/AJ-E1612/dev-046/opencode-4-usage-session.jpg. Fix: with no known limit, words without limit bars ("This session: 32k of the 32k AI Juggler saw on this App"), or no card (D-127).
- Dev reads Grok Build CLI · R3 as "needs its sign-in": its lab sign-in no longer holds on that folder (the maintainer's sign-in list).

## 0.4.7 nightly walk V1 (Pools; P1, P5, P7, P10, PC1 labs; 0.4.7 Dev build): Opus-read
- PASS: setup summary rows whole and in view, no stray "More below"; the pool header's ⋯ (Rename, Delete); the settings panel scrolls by wheel to Delete pool…; the List view's role control (SK-13); the grid holds still on Make, the new pool last (E3014-1); "Personal / Account · Member" on the card (SK-9); P7: no lone No-lead card, Who's in "client (in Client, sealed)", the App node reads "App · Member" (E3014-9); P10: bordered "Use a template" after the first member, gone at 7/7 (SK-7); PC1: centred diagram (SK-3).
- SK-12 "FAIL" was the brief's wrong place: the wording lives in the settings panel ("Everyone in client, now and new, is: Not in the pool"); the count shows only with a role set: recheck the count with a role (batch 2 check).
- Batch 2: (1) the wizard's left rail and its Cancel take no accessibility press ("nothing to press there"); U1 found the wizard's modal logic may make its own rail inert: look there; (2) the header ⋯ sits after the title chips, not at the title row's right end; (3) a template card's body takes no press, only its radio (Next without a pick skipped the template and made a pool with no project); (4) the Pools grid reorders after a delete; (5) the Tasks skeleton couldn't be caught by the driver (first frame at 228 ms already "No tasks yet").

## 0.4.7 nightly walk V2 (PD4, K3, A5, N5, PM4, P7 labs; 0.4.7 Dev build): Opus-read
- PASS: Processes drops the stopped left-behind, Clean up says "0 to clean up … Nothing needs you" (SK-10 on Processes); K3's asking session shows "Bash command npm test … Do you want to proceed?" and Home quotes "Bash: npm test"; This Mac: one line "Room counts memory, swap, the processor, heat and disk." and a closed "When does it turn tight?"; A5: greyed Continue with "It has no conversation to carry over yet." and Home agrees (SK-2); N5: folder from the api project, no Suggested on out-of-room Work (SK-6), the toast "Stopping api · started 6:14 PM. … Undo keeps it running."; PM4: the switched session keeps "Refactor the settings screen" and "continues from Claude Code CLI · Work · work" opens the original (SK-11); pages say "Running in AI Juggler · This Mac" at once, one sidebar row each (E2311-1, E2025-2 seen).
- P7 step 3 PASS without a model turn: the sealed session's `--settings` include `…/overlays/sealed/<session>/juggler-sealed-settings.json` with `"crossSessionInbound": "refuse"`, and SendMessage and ListAgents denied.
- SK-5 in K3: "It stopped at 6:09 PM; Claude Code didn't say why." (the seeded crash has no exit code; the honest words for it). PASS with the seed's limit.
- Batch 2: (1) SK-10b: This Mac's Overview tile still says "1 process left running after its session closed" + Clean up 40+ s after the stop (Processes updates, the Overview doesn't); (2) the New session sheet still shifts ~50 px when its warning leaves (N5; ~20 px in P7); (3) once, This Mac's Overview showed an empty Memory card, "Processor — Not measured yet" and "No AI tools are running" for 40+ s with two sessions running (not reproduced); (4) Suggested needs a measured room, so an unmeasured App with room is only offered inline (by U2's design; check the words read well).
- Not walked: Open in terminal's door (it opens Terminal), the 1024×680 window (no resize verb).

## From the 0.4.7 persona runs (batch 2)
- SK-23: a running session's page opens on a blank Live terminal: what it printed before the page opened isn't replayed (K3's stand-in asks once and waits; V2 saw it when the page was already open). Check with real Claude Code on aij-test (its TUI redraws on a size change) before calling it the product's; replay the kept screen on attach either way.
- SK-24: a session page's header says "started 1s ago" for a session started minutes before (M2 at 6:25 for a 6:22 start; LA saw the same).
- M5: the parked session's "resets in 3m" didn't change across a minute (Cancel, then Home): confirm the window wasn't covered (AJ-E805), else the countdown isn't live.
- D1: "Plenty of room" green with swap 8.2 of 9.0 GB (the rule needs ≥ ¼ of RAM in swap too): the words, or the rule, mislead.
- SK-23 note: D2 opened the asking session from the sidebar and saw its question; M2 opened it from Home's 12-hour list and saw a blank terminal: check whether the way in (or the moment) decides the replay.
