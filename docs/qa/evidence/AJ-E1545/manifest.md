# Evidence manifest — AJ-E1545

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1545, the Welcome leads into a
guided setup wizard, and every brand mark is the real app icon (P0).
Status: **IN-REVIEW** (built 2026-11-23; the Sonnet newcomer-persona walk
and Opus's check of this critique come next).

Captures (1280×800, both themes, generic fixtures only, recorded by
`ui/tests/clips/setup-evidence.spec.ts`; each opened and checked by eye):
[Welcome](docs/qa/evidence/AJ-E1545/welcome-dark.png),
[Your apps](docs/qa/evidence/AJ-E1545/step1-your-apps-light.png),
[Mac access](docs/qa/evidence/AJ-E1545/step3-mac-access-dark.png),
[a grant seen and a skip](docs/qa/evidence/AJ-E1545/step3-mac-access-allowed-and-skipped-light.png),
Meet Bot with [Apple's model](docs/qa/evidence/AJ-E1545/step4-meet-bot-apple-dark.png),
[the local offer](docs/qa/evidence/AJ-E1545/step4-meet-bot-noapple-light.png) and
[no engine](docs/qa/evidence/AJ-E1545/step4-meet-bot-nolist-light.png),
[Keep running](docs/qa/evidence/AJ-E1545/step5-keep-running-dark.png),
[the end, with the tour](docs/qa/evidence/AJ-E1545/end-take-the-tour-light.png).
Every file exists as `-dark` and `-light`.

## What exists now

| Claim | Verdict | Proof |
|---|---|---|
| Welcome leads with "Let's get you set up" and one primary, Get started, opening the wizard; "Make it smoother" left it | PASS | `welcome.test.tsx`, `home.test.tsx`, `app-find-accounts.test.tsx` |
| Bot leads "What it does for you", full width, Beta tag, its clip of a cited answer; D-144's words | PASS | `welcome.test.tsx`; clip `what-bot` |
| A rich-mode window sheet: sticky header (the real icon), the step column (Your apps · Sign in · Mac access · Meet Bot · Keep running), sticky foot, Not now and Skip for now on every step, left and resumed where it was | PASS | `setup-wizard.test.tsx` |
| Your apps: "Which AI apps do you use?", one group per tool with the tool's icon (`ToolLogo`, read at run time) and select-all, plain status words, no badges or paths up front, not-set-up tools in one closed unticked group, copies under More options, the tools' terms closed at the foot, Bring in N = the ticked rows | PASS | `first-run.test.tsx`, `first-run-pf1.test.tsx`, `tools-catalog.test.tsx` |
| Mac access on AJ-E1547's registry and gate: one card per needed prompt with the picture of the system's dialog and the button to click, why, if declined, where to change it later, its clip; Allow primes it through the gate; ✓ only when the system says so; Skip for now per card; a line when the dialog may be behind the window | PASS | `setup-wizard.test.tsx` |
| Meet Bot after the permissions, before Keep running: what Bot does (answers, cites, never acts), then the shared EngineSetup (AJ-E3823) with its Skip for now; nothing downloaded without a click | PASS | `setup-wizard.test.tsx` (Apple, local offer, no engine, ready) |
| Your apps pairs a desktop profile and its folder into one row by AJ-E2327's evidence (never by name); an uncertain one asks | PASS | `first-run-pairing.test.tsx` |
| F2: an uncertain pairing offers "Choose its folder…" for an added App and, before Add, for a profile not added yet: the pick is checked as an App's (a non-tool folder and one its running sessions contradict are refused), kept with the offer, nothing added or brought in (D-84), linked by its Add; the row then follows the folder chosen (2026-11-23, c59a5031d) | PASS | `places::link_tests` `a_folder_chosen_before_add_is_kept_and_linked_by_add`, `proof_refuses_a_pick_before_add`, `a_declined_offer_takes_no_pick`; `first-run-pairing.test.tsx` "an uncertain pairing offers Choose its folder…, for an App and before it's added (F2)", "a folder chosen before Add moves the profile to that folder's row, sure (F2)" |
| R8: Step 2, Sign in: a tool that shows a page gets a card, not its terminal: "Open <tool>'s sign-in page" as the one primary, the code large with Copy and "Check the page shows this code, then approve" (or a field for a code the page gives back), a calm "Waiting for you to approve…" turning into ✓ signed in; the terminal under "Show details", collapsed; a sign-in that can't be read keeps its terminal with one plain line; each row the tool's icon and a Terminal chip; the header "Each opens its own sign-in page; AI Juggler never sees your password"; each tool's output read in its adapter with a fixture test per tool (2026-11-23: 00a379508, c6afbf9c2, 252f1b311, f6187543f, 85d1f8cf2) | PASS | `sign-in-card.test.tsx` (page + code with Copy, paste-back, waiting, ✓ signed in, ended early with last lines and Try again, unreadable → terminal with one line, Show details collapsed, the panel's Terminal chip); `first-run.test.tsx` (the step's header words); `sign_in_finish.rs` (8 adapters' fixtures); `no_bare_tools` |
| Keep running: opening at login only where the build can, keep awake (on when plugged in) | PASS | `setup-wizard.test.tsx` |
| The large mark is one hand-drawn SVG of `icons/icon.png`; the small mark the same arc and balls | PASS | `components/AppMark.tsx`, `IconJuggler`; captures |

## Not done here, and why

- (Done since, 2026-11-23: the sign-in card, R8 above, with AJ-E1553;
  "Is this right?" now offers "Choose its folder…" before and after Add,
  F2 above.)
- The Sonnet newcomer-persona walk and its captures of the sign-in card
  (both themes) come with the next RC.

## Design critique (Opus, against DESIGN-SYSTEM and the d126 kit)

- Matches: FlowLayout window sheet as the kit's who-light (steps column with
  values, eyebrow "Step N of M" in the accent, 22 px titles, foot with Cancel
  left and the primary last naming where it goes, "Finish setup" on the last).
  One filled primary per view; Skip for now is secondary, Not now ghost.
  Permission cards: icon tile, title ink, help ink-2, the system's words in a
  surface-2 panel, the grant as a quiet ✓ (L1) with a success edge.
- Fixed after the first critique: Your apps' title is the steps' 22 px;
  its actions (Bring in N, Set up a new app instead, Add from another
  folder…, Look again) sit in the wizard's sticky foot; the 32 px header
  mark has an edge that stands off the dark surface.
- Still weak: the permission clips are page loops (the focus clip shows an
  App page opening), not the effect itself.

## Usability reasoning, re-checked on the built screens

1. Newcomer, a guided path with an end: matches (Get started, 5 named steps).
2. One sentence before any control; the system's dialog and what to click:
   matches; Bot's sizes and free disk come with AJ-E3823's component.
3. Setup in one place, each step pointing to where it lives later: matches.
4. Nothing to do → left out or one line with Skip; leave and resume: matches
   (the access step leaves when nothing is missing; Not now resumes there).
5. Each prompt primed before Allow: the cards say it; the gate is AJ-E1547's.
6. Watch-fors: no system dialog before its card (the wizard calls nothing
   until Allow); dialog behind the window gets its line; every step skippable;
   no download without a click (no Download here); ✓ only when the system
   reports it; the mark matches the Dock icon (both themes).

## Usability reasoning for R8 (the sign-in card), re-checked on the built UI

1. A newcomer signing in to each app expects the tool's own page, not a
   terminal: matches (one primary, "Open <tool>'s sign-in page"; the
   terminal only under Show details, closed).
2. One sentence before any control, no internal words: matches ("Check
   the page shows this code, then approve", "If the page gives you a
   code, paste it here for <tool>:"; no "provider", "lease" or ids).
3. One card for the wizard's step and a sign-in's own page (the rail's
   Sign-ins): matches (`SignInCard` in both, fed by one list).
4. Nothing to read → no fake card: matches (the terminal with "<tool>'s
   sign-in runs below: follow its steps there. AI Juggler notices when
   you're done."); signed in → "✓ <tool> is signed in" with Done; ended →
   its last lines, Try again, Dismiss.
5. No system prompt unannounced: matches (opening a web page raises
   none; recorded in `docs/design/os-prompts.md`; the address is the one
   the adapter read, opened by the backend).
6. Watch-fors: a code too small to compare (it's `text-stat`, spaced, with
   Copy), a terminal that opens by itself (collapsed by default), an
   empty terminal after an early end (never: its lines or one sentence),
   a second primary (Copy and Send are secondary): all match.

## Close-out (0.5.1, 2026-11-23, Opus)

Each test named below was found by name and run, targeted, on `release/0.5.1` (vitest by file; cargo through `scripts/heavy.sh`, `--lib` with module filters). An OPEN line says where it closes.

R1: PASS — vitest `components/welcome.test.tsx`, `components/setup/setup-wizard.test.tsx` ("the indicator: Your apps · Sign in · Mac access · Meet Bot · Keep running…"), `app-find-accounts.test.tsx`, `screens/home.test.tsx` (994580b18).
R2: PASS — vitest `screens/first-run.test.tsx`, `first-run-pf1.test.tsx`, `first-run-pairing.test.tsx`, `tools-catalog.test.tsx`; cargo `places::link_tests::pairings_pair_each_profile_by_evidence_and_add_nothing`, `a_folder_chosen_before_add_is_kept_and_linked_by_add`, `proof_refuses_a_pick_before_add`, `a_declined_offer_takes_no_pick` (35e54e560, c59a5031d). 0.5.1 F35: every desktop profile is a row in its tool's group (Desktop chip, one row with its folder when paired, unticked until the user ticks it, counted in Bring in N; no separate block): vitest `first-run-pairing.test.tsx` (the backend's real wire shape, failing before), cargo `places::link_tests::pairings_name_the_tool_and_the_folder_as_the_scan_lists_it`, `pairings_leave_out_an_offer_whose_profile_folder_is_gone`; gallery `journey-first-run.spec.ts` expectations updated, not run (c387698e7). PL1 C3 re-walk to confirm.
R3: OPEN — critique round and PL1 step C9. The cards, the gate, ✓ only on the system's word, and Skip for now are tested (`setup-wizard.test.tsx` "one card per prompt, through the gate…", "isn't a step when nothing needed is missing"). "A short clip of the effect": PASS (0.5.1 F3) — `perm-focus` is re-made with `scripts/record_clips.sh` (`tests/clips/record-clips.spec.ts` recipe `perm-focus`): AI Juggler's real window on the gallery's fixtures, Go to it on the "Fake Desk" session, and that session's own window comes to the front (both themes, 28 KB each; `tests/bundle/clips-size.test.ts` passes; every frame opened). Still open: the live grant seen on the real Mac (PL1 C9, K1).
R4: PASS — `setup-wizard.test.tsx` "says what Keep it running keeps on", "offers opening at login only where it can, keeping awake on, then the tour".
R5: PASS — `welcome.test.tsx` (Bot's lead card, Beta chip); `setup-wizard.test.tsx` "Skip for now is the engine setup's; Next steps down beside its own primary"; `components/ai/engine-setup.test.tsx` (ready, local offer with size and free disk, no engine possible with Skip) (753042cb9).
R6: PASS — `setup-wizard.test.tsx` "…keeping awake on, then the tour"; `components/tour/tour.test.tsx` "the host shows the offer only where it's allowed, and Show me starts those steps".
R7: OPEN — critique/baseline round: no test compares the hand-drawn SVG mark with `icons/icon.png`, so it is a judgement on screenshots in both themes (PL1 C2's header mark too).
R8: PASS — vitest `components/sign-in-card.test.tsx`; cargo juggler-adapter `sign_in_page` tests (6) and juggler-adapters `--test sign_in_finish`, one fixture per tool (`claude_code_…`, `codex_…`, `grok_build_…`, `kimi_code_…`, `opencode_…`, `zcode_…`, `muse_code_…`, `hermes_…`) (00a379508, 252f1b311).
