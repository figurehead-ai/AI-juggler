# Evidence manifest — AJ-E2105

Task: `docs/tasks/E21-desktop-first.md` AJ-E2105, Home: what needs you
and your apps. Branch `e21-rail`. Status: **IN-PROGRESS**: built and
tested below, Home empty, typical and busy captured in both themes on
this Mac; open: the container gallery run (the maintainer's), the load
gate on Home (`just load-gate`, the maintainer's run), and the Needs you
card's last-message bubble (the card takes it as `bubble`; Home doesn't
read transcripts yet, AJ-E2108's reader serves the session page).

## What exists now (`ui/src/screens/Home.tsx`, `components/HomeApps.tsx`, `HomeNeeds.tsx`, `HomeRunway.tsx`, `lib/home-needs.ts`)

1. **The heading** is one sentence about what needs you ("2 things need
   you"; with nothing: an account run out, else "All clear — nothing
   needs you", else "Ready when you are"), 24 px bold, the other counts
   under it as links: "n open" (the rail's first row), "n inactive" and
   "n ended today" (turn the rail's views on), "n in <app> hidden"
   (Settings).
2. **Needs you** holds every kind, each with its own primary: a session
   asking (Go to it ↵; Continue on another account…), something to
   review (Go to it), a limit reached (Continue elsewhere; Wait parks
   it), a failure (See what happened), a sign-in (Sign in), a Mac under
   pressure (See what's using it), a session working the Mac hard (Take
   a look), processes left behind (Clean up), and the monthly caps. The
   first is large: title, "project · app · profile · account", its two
   buttons. A parked session keeps its own section (AJ-E1021); "Pick up
   where you left off" stays.
3. **Your apps**: one tile per running app and profile (AI Juggler's own
   left out: it's here): the mark, the app, the profile, its sessions, a
   needs-you badge, its account's runway line and meter, Open <app> (the
   app forward, never launched); six, then "+n".
4. **Runway by account**: each subscription once (AJ-E2109), its email,
   "% left", a meter, "Profiles: …", when it resets or "Measure its
   limits ›".
5. **Gone** from Home: its Running list (the rail and the tiles), its
   old Runway (by account now) and its This Mac panel (the top bar's
   line; This Mac's page has the rest).

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 one sentence, counts as links | `cd ui && npx vitest run src/screens/home-places.test.tsx` ("one sentence…"); `src/screens/home.test.tsx` | "3 things need you"; links turn the rail's views on | pass | PASS | e1baac01 | 2026-10-24 | agent |
| Req.2 every kind, each with its primary | `home-places.test.tsx` ("Needs you holds every kind…"); `src/components/this-mac.test.tsx`; `src/machine-state-agreement.test.tsx` | session, limit, leftovers, sign-in, Mac, heavy: each its button | pass | PASS | e1baac01 | 2026-10-24 | agent |
| Req.3 a tile per app and profile, six then +n | `home-places.test.tsx` ("Your apps…") | 4 tiles; 8 places → 6 + "+2" | pass | PASS | e1baac01 | 2026-10-24 | agent |
| Req.4 runway by account with profiles; Not measured yet and the way to measure | `home-places.test.tsx` ("Runway by account…", "…groups the profiles signed in to one account") | "Profiles: Work Max, Personal" on one account; "Measure its limits ›" | pass | PASS | 7180ed32 | 2026-10-24 | agent |
| Perf: Home with 50 sessions | `home.test.tsx` ("renders 50 sessions well inside 300 ms") | < 300 ms | pass | PASS | e1baac01 | 2026-10-24 | agent |
| By hand: preview, both themes | `vite --port 5291` with the places data | the canvas's Home; Open on a tile says what it did | as expected | PASS | e1baac01 | 2026-10-24 | agent |
| Visual: empty, typical, busy; both themes | `AIJ_PW_PORT=4391 npx playwright test tests/visual/places-gallery.spec.ts` (this Mac) | axe 0 critical/serious; looked at | pass | PASS | c75cc9d5 | 2026-10-24 | agent |
| The load gate and the container gallery | `just load-gate`; `just visual-gate` | within budget; baselines approved | not run (the maintainer's) | UNPROVEN | — | 2026-10-24 | agent |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (none: fixtures only)
