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
| Keep running: opening at login only where the build can, keep awake (on when plugged in) | PASS | `setup-wizard.test.tsx` |
| The large mark is one hand-drawn SVG of `icons/icon.png`; the small mark the same arc and balls | PASS | `components/AppMark.tsx`, `IconJuggler`; captures |

## Not done here, and why

- **The friendly sign-in card (step 2) and AJ-E1553's "signed in?" signal
  and rail clean-up**: adapter work in `crates/juggler-adapters` (a
  sign-in prompt parse per adapter, fixture tests), not started in this
  batch. Sign in is today first run's own sign-in step, in place.
- **"Is this right? Change"** on an uncertain pairing: the row asks and
  points to the App page; choosing the folder from here needs AJ-E2327's
  chooser.

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
