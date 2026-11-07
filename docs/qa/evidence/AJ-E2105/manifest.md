# Evidence manifest — AJ-E2105

Task: `docs/tasks/E21-desktop-first.md` AJ-E2105, Home: what needs you
and your apps (R3 as amended by D-63, 0.3.3). Re-audited against the code,
its tests and the current gallery on `release/0.4.1`, 2026-11-07. Built in
E21 (ac0e41dc0, 0.1.3), the 0.2 kinds with E18/E19 (f6fb096cb), the
kept-alive tiles in 0.3.3/0.3.4 (2251ce94d), the cards lined up in 0.4.1
(97f28fbd5).

Status: R1, R3 and R4 evidenced; **R2 is a GAP**: every kind is built,
but Home never passes the first need its last-message bubble, the paused
chain's primary is "Deliver"/"Resume" rather than "Continue", and no test
renders the 0.2 kinds inside Home's Needs you (only each component alone).

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 the heading is one sentence about what needs you, the other counts as links | `cd ui && pnpm -s exec vitest run src/screens/home-places.test.tsx`: "one sentence about what needs you, the other counts as links that turn the rail's views on"; `src/screens/home.test.tsx`: "its heading counts the rows under it, even while the count it was told is behind (the 0.2 walk, F4)", "all clear while something runs; ready when nothing does"; gallery `_gallery/94-places/home-typical-dark.png` | "3 sessions need you"; "7 open · 1 inactive · 1 ended today", the links turn the rail's views on | pass; the capture (opened) shows "3 sessions need you" over "8 open · 1 inactive session · 1 ended today" | PASS | ac0e41dc0 | 2026-11-07 | Claude (self) |
| R2 Needs you holds every kind, each with its own primary (amended: a paused chain's word is Deliver; no last-message bubble on Home) | vitest `screens/home-places.test.tsx` "Needs you holds every kind, each with its own primary" (session, limit, leftovers); `screens/home-agent-needs.test.tsx` "holds the agents' messages, asks, paused chains and prepared launches, each with its own primary, and counts them" (Allow once, Deliver, Read, Start session inside `home-needs`; the heading counts 4); `pages/tasks/tasks.test.tsx` "an agent's request for a session: you start it on your pick, or leave it" | every kind on Home with its primary, counted | pass | PASS | 407be729a | 2026-11-07 | Claude (self) |
| R3 Your apps: a tile per running app and profile (count, needs badge, runway, Open), six then "+n"; amended: a kept-alive one that's down has a tile (why, Start), a running kept-alive one says so | `home-places.test.tsx`: "Your apps: a tile per running app and profile, with its count, needs badge and Open; six, then how many more", "a tile says its room once: nothing when unknown, and no Account name that repeats its profile (0.4 design review, 83 and 49)", "the cards line up (0.4.1): …"; `src/screens/settings/keep-alive.test.tsx`: "has a tile with why and Start, which starts it"; gallery `_gallery/94-places/home-cards-dark.png` | 4 tiles; 8 places → 6 + "Show 2 more apps"; seven shown whole; "Not running · keep alive starts it" + Start; "Running · kept alive" + Open | pass; the capture (opened) shows Work Max with the kept-alive mark, "60%/70% left" meters, "1 needs you" badges, and two down tiles "Not running · keep alive starts it" / "seen 9 days ago" with Start | PASS | 2251ce94d | 2026-11-07 | Claude (self) |
| R4 Runway by account lists subscriptions with their profiles; "Not measured yet" with the way to measure | `home-places.test.tsx`: "Runway by account: each with its profiles; the way to measure one that isn't", "Runway by account groups the profiles signed in to one account"; `home.test.tsx`: "runway: no account reading → the newest session's own reading, then 'Not measured yet'"; gallery `_gallery/94-places/home-typical-dark.png` | "Apps: Claude Code · Work Max, Claude Code · Personal" on one account; "Not measured yet" + "Measure its limits ›" | pass; the capture shows both accounts with their Apps and "Not measured yet · Measure its limits ›" | PASS | ac0e41dc0 | 2026-11-07 | Claude (self) |
| Gate: vitest; the load gate; visual both themes | `cd ui && pnpm -s exec vitest run src/screens/home-places.test.tsx src/screens/home.test.tsx src/components/this-mac.test.tsx src/screens/settings/keep-alive.test.tsx src/pages/messages/messages.test.tsx src/pages/tasks/tasks.test.tsx` (run 2026-11-07); `home.test.tsx` "renders 50 sessions well inside 300 ms"; Playwright `tests/visual/places-gallery.spec.ts` "Home, nothing open", "Home, typical: what needs you, your apps, runway by account", "Home, the app cards line up: …", "Home, busy" (both themes) — runs in CI / the gallery gate; `just load-gate` — runs in CI | 6 files pass; < 300 ms | 6 files, 97 tests pass; the Playwright and load gates not run here (CI / the gallery gate) | PASS (vitest); CI for the rest | 97f28fbd5 | 2026-11-07 | Claude (self) |

## What's missing for R2 (to close the GAP)

1. **The last-message bubble.** `ui/src/screens/home-cards.tsx`, the `needs`
   card: pass `bubble` to `HomeNeeds` for the first session need, from the
   session page's last-message reader (AJ-E2108; the conversation feed),
   showing only the message text the reader already exposes. Test, in
   `home-places.test.tsx`: with a fixture conversation whose last message
   is known, the `[data-first='true']` need contains it; with none, no
   empty bubble.
2. **The paused chain's primary.** `ui/src/components/MessageNeeds.tsx`:
   R2 names it "Continue"; today the hop-limit card is "Deliver" and the
   paused pool/thread rows "Resume messages"/"Resume". Either rename the
   primary or amend R2 (the maintainer's call: a wording difference only).
3. **A Home-level test** for the 0.2 kinds: in `home-places.test.tsx`, seed
   `__AIJ_MESSAGES__` and the task needs and assert `message-need-waiting`,
   `ask-card` and `task-need-launch` render inside `home-needs`, each with
   its primary, and that the heading counts them.

## What exists now (`ui/src/screens/Home.tsx`, `home-cards.tsx`, `components/HomeApps.tsx`, `HomeNeeds.tsx`, `HomeRunway.tsx`, `lib/home-needs.ts`)

1. **The heading** is one sentence about what needs you ("3 sessions need
   you"; with nothing: an account run out, else "All clear — nothing
   needs you", else "Ready when you are"), the other counts under it as
   links: "n open", "n inactive" and "n ended today" (turn the rail's
   views on).
2. **Needs you** holds every kind, each with its own primary: a session
   asking (Open its page ↵, with ↗ to its app; Continue in another app…),
   something to review, a limit reached (Continue in another app…; Wait
   parks it), a failure (See what happened), a sign-in (Sign in), a Mac
   under pressure, a session working the Mac hard, processes left behind
   (Clean up), the monthly caps; agents' messages and requests (the
   approval card: Allow once / Deliver), messages waiting (Deliver), paused
   pools and threads (Resume), an agent's request for a session (Start
   session). Chips narrow the list when two kinds or more are present.
3. **Your apps**: one tile per running app and profile (AI Juggler's own
   left out), its sessions, a needs-you badge, its account's room, Open;
   six, then "Show n more apps" (seven shown whole). Remembered apps not
   running have a tile after them, kept alive first (why, Start); a
   kept-alive one carries its mark.
4. **Runway by account**: each subscription once, its email, "% left", a
   meter, "Apps: …", when it resets or "Not measured yet · Measure its
   limits ›".

## Binary artifacts

None added; cited: `_gallery/94-places/home-{empty,typical,busy,cards}-{dark,light}.png`
(typical and cards opened and looked at, 2026-11-07).

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (repo paths only)
- [x] Transcript excerpts scrubbed of personal content (none: fixtures only)
