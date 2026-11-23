# Evidence manifest — AJ-E1548

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1548, the 0.5.0 usability
audit's findings (`docs/qa/0.5.1-usability-audit.md`) fixed in 0.5.1: one
name per thing, no rows nobody can use, every mark and limit said
plainly, the providers' terms never open, AJ-E1547's last three calls
through the OS-prompt gate, and five of the seven ruled behaviour
changes. Status: **TODO (0.5.1)**, 2026-11-23: R9 and R10 are partial
(below), so the header stays open.

Commits: f68da7612 (#8 #10 #11 #12 #28 words), 026fd59ef (#17 #18),
03b196314 and 17523730b (#19-#22), 2b787c737 (#1 #23 tooltip #24 #25
#26), 868d1786f (R6 gate routes, Watches priming), 949ea180f (R11),
57448a6a6 (#13), ad0d02fac and 1ebb9b80b (#14), a93bd69bb (#15), 8145e17f2
(#9), 3769c0098, ef80a23a6 and 680856f51 (#3 #4 #5 #6 #7), b5c71f345
(notes, trust guide), 567b67ed8 (terminology lint allows PAT-SER-7's
label).

No captures: each change is proved by tests against the built
components and the backend (generic fixtures only). The visual gate and
Opus's critique in both themes are still to run on the built app.

## Requirements

| R | Claim | Verdict | Proof |
|---|---|---|---|
| R1 | #1 the Bot page's memory count matches its list: one shared read for the tab, the side card and the list, refreshed once on Remember, Forget and Undo | PASS | `bot-memory.ts` on `sharedRead`; `bot.test.tsx` "AJ-E1548 #1: the remembers tab's count is its list's, after Remember too (one read for the page)" (2b787c737). Confirm on the 0.5.1 build in the next walk. |
| R2 | #3 #4 #7 Models lists only the jobs a model passes today, with one quiet line for the rest; Attention hints gone from AI › Jobs; the lead says only what ships | PASS | `models-view.ts` `jobsToday`, `JOB_ROWS` without attention; `models-pane.test.tsx` "AJ-E1548 #3 #4 #7: …" (3769c0098) |
| R3 | #6 one name for Bot's switch in Bot and Settings: "Bot’s AI", the name AJ-E3823 settled | PASS | `models-words.ts` `JOB_NAMES.chat`; `usability-words.test.tsx` "Settings and Bot both call it Bot’s AI" (680856f51) |
| R4 | #8 "Finishes" means a session ending on its own (Notifications, the feed); a watch's turn end reads "Its turn ends" / "finishes its turn", backend and preview alike; #10 the bell says "Needs you · N"; #28 Processes' tab says "Needs you" | PASS | `watches/words.rs` FINISHES; `watch-echo.json` shared cases (Rust `the_echo_says_the_shared_cases_word_for_word` and the TS echo); `usability-words.test.tsx` "#8 #10 #28"; `notifications.test.tsx`, `notification-views.test.tsx` (f68da7612) |
| R5 | #11 the top bar says "This Mac: fine" / "This Mac: getting tight", "room" kept for Accounts; #12 a level's mark sits after its count, and the legend draws the rail's marks and says so | PASS | `MachineStatusLine.tsx` `levelWords`; `rail-groups.tsx`; `LegendLevels.tsx`; `usability-words.test.tsx` "#11 #12"; `machine-state-agreement.test.tsx` (f68da7612) |
| R6 | #16 Keychain in AJ-E1547's registry (done there); the first watch alert's notification permission primed at the Watches form's Watch button (Not now still saves); `goto.rs` `osascript_with` through `os_prompt::script(…, Background, …)` (System Events, or Terminal/iTerm for a tab script), `goto.rs` `send_link_to` and `handoff.rs` `send` through `os_prompt::send_link(APP_LINKS, user(), …)`; their allowlist lines removed | PASS | `goto/tests.rs` `each_script_waits_on_its_own_apps_prompt`; `--lib features::conversations` 106 and `features::places` 115 pass; `watches.test.tsx` "AJ-E1548 #16: Watch primes the notification permission first …"; `scripts/os_prompts.allow` holds no entry; `os_prompt_lint.py` OK and `--selftest` OK (868d1786f) |
| R7 | #17 one machine word in the Unlink sheet (`machine-words.ts`); #18 Find my Macs hidden until it ships, its choice kept and saved | PASS | `FigureheadCloudParts.tsx` `shownFeatures`; `settings-screen.test.tsx` (the consent screen); `usability-words.test.tsx` "#17" (026fd59ef) |
| R8 | #19 each "mark all" says what it clears; #20 the digest counts sessions and names twins "api (1)", "api (2)"; #21 the scope picker reads "Showing: …" with the picked one's kind; #22 "While you were away" and "the digest" kept apart in the guide and the 0.5.1 notes | PASS | `feed-words.ts` `MARK_HINTS`, `digestHeader`; `home-feed-digest.test.tsx` "AJ-E1548 #20 …"; `usability-words.test.tsx` "#19 #21"; `docs/guide/notifications.md`, `docs/releases/0.5.1.md` (03b196314, b5c71f345) |
| R9 | #23 the Floor says what it is (tooltip, a first-open line); #24 floors named by what is on them; #25 the watch limit only near it; #26 Bot's memory says which watches it counts and links to them | UNPROVEN | Built: #23 tooltip, #24 "APPS 1–10" (`scene::tests::a_floor_is_named_by_the_apps_on_it`), #25 (`usability-words.test.tsx` "the count beside Watches shows from 40 of 50"), #26 (`bot.test.tsx` "AJ-E1548 #26 …") (2b787c737). Left: #23's first-open line inside the native Floor window (juggler-floor overlay work, with the visual gate). |
| R10 | The seven ruled behaviour changes | UNPROVEN | Built, each tested: #15 Check for configuration updates (`remote_config` `check_for_configuration_updates_is_on_by_default_and_the_switch_holds`, `privacy-page.test.tsx` "AJ-E1548 #15 …", three new trust lines checked by `tests/trust.rs`) (a93bd69bb); #5 job switches only on Models, Jobs sub-pane gone (`settings-sections.test.tsx`) (3769c0098); #9 the bell's latest five with See all on Home (`notification-views.test.tsx` "AJ-E2503 R3, AJ-E1548 #9 …") (8145e17f2); #13 the session banner keeps facts with Remind/Snooze and no Switch account… (`banner-band.test.tsx`, `switch-not-yet.test.tsx`) (57448a6a6); #14 Remind me makes a one-time Room again watch (`banner-band.test.tsx` "AJ-E1548 #14 …") (ad0d02fac). Left: #2 ("This is wrong" removes the answer from the conversation's context, Undo, Ask again) and #27 (a pool's setup checklist owns the rules while open). |
| R11 | Providers' terms never shown open: Settings › Privacy, setup and Add account keep them behind a closed "Providers’ terms"; the switch and move dialogs and an App's page show no terms text; the guide links | PASS | `privacy-page.test.tsx` "lists each tool's terms …" (closed first) and "Add account shows it for the tool being added" (closed until opened); `first-run.test.tsx`; `privacy-gallery.spec.ts` opens it first (949ea180f, b5c71f345) |

## Usability answers (the entry's six), matched on the built code

1. Daily users read each name as one thing: "Its turn ends" vs "A session finishes", "Needs you" in the bell and Processes, "This Mac: fine", "Bot’s AI", "sessions" in the digest. Match.
2. Every mark explained: the legend says where the marks sit; each mark all has its line; the watch limit shows only from 40 of 50. Match.
3. Same surfaces, clearer, no second door: the Jobs sub-pane, the bell's 30-day list and the banner's Switch account… are gone. Match.
4. Rows nobody can use hidden: jobs no model passes (#3), Find my Macs (#18). Match.
5. Keychain (AJ-E1547) and the first watch alert's notification permission primed at Watch; Go to it's and a hand-off's calls through the gate. Match.
6. The watched-for surprises: count vs list (#1), one switch two names (#6), "Plenty of room" beside "1 account low" (#11), twin rows (#20): each has its test. Match.

## Checks run (2026-11-23)

- vitest: the touched files (watches, bot, notifications, home feed, privacy, first run, settings, models, banners): all pass.
- `tsc -b --noEmit` clean; `ui_lint.py`: no finding in AJ-E1548's files (others' files have their own).
- cargo: `--lib watches` 65, `features::conversations` 106, `features::places` 115, `command_types` 6, `remote_config` 12, `--test trust` 18, `-p juggler-floor` `a_floor` 6 pass; clippy `-p juggler-app -p juggler-floor --all-targets -D warnings` clean; `cargo fmt --check` clean on both.
- `juggler-floor` `colour::tests::the_mark_colours_match_the_main_windows` fails on HEAD before this work (AJ-E1550's mark hues vs `colour.rs`), not this item's.
- Static: `os_prompt_lint.py` (and `--selftest`), `keychain_lint.py`, `plain_words_lint.py`, `private_data_lint.py` OK; `terminology_lint.sh`: only `PoolCards.tsx:31` (AJ-E2314's) remains.
