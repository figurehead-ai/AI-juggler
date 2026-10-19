# Evidence manifest — AJ-E1014

Task: `docs/tasks/E10-derived-workflows-and-beta.md` AJ-E1014 (Home and
the one rail, D-25). Status: **DONE-EVIDENCED**. 2026-10-19, reviewer
claude-opus.

## What exists now

- **Top bar**: Home, Accounts, History, Settings, More (Triage, Overview,
  Usage, Fleet, Cleanup, each with a one-line hint) and New session.
  Cleanup shows as its own pill while it has alerts; Fleet once a peer is
  paired. The More menu now paints above the page (before, the page
  covered every item but the first) and closes on Escape or a click
  elsewhere.
- **One rail**: projects with their sessions (a project header opens the
  project), search, Needs you · Running · All, and a Filters button that
  holds Ended, sort, group, provider, account, machine and project, says
  "Filters (some on)" when any is on and offers Reset. Idle projects are
  listed when nothing is filtered; "No sessions yet" and "No sessions
  match" are the empty states.
- **A row**: state icon and words, branch or folder, time, the account's
  colour. Only the exceptions are added: quota at 25 % or less, parked,
  uncommitted files ("3 changed"), a sign-in to check, a stale value
  ("stale - 3m"), a desktop-app session ("app"); the time yields first
  when space runs out. Compact is one line, Comfortable two.
- **Home**: the headline ("5 sessions need you", "All clear — nothing
  needs you", "Ready when you are"), what changed while you were away
  (AJ-E1006's summary, moved here from the empty inspector), Needs you
  (one sentence and one action each), Running, Runway (the tighter
  measured window per account; with no account reading, the newest
  session's own quota line; otherwise "Not measured yet"), Pick up where
  you left off, and one plain next step when there are no accounts.
- **Session view**: a short summary (state, ended time, account with
  "% left" only when low, identity, model, branch, park); the rest under
  Details. Footers name their source in words ("from this Mac's process
  monitor"); announcements say what happened ("waiting for your reply").
- **Density** in Settings › Appearance, each choice described.
- The old "fleet at a glance" panel is retired: nothing selected means
  Home.

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 top bar, More, pills | the whole Playwright suite navigates through `goTo` (direct or via More); `home-gallery.spec.ts` "More menu": every item in view, closes on an outside click | PASS |
| Req.2 one rail, quick filters, Filters | vitest `sorting.test.tsx` (Needs you, No sessions match, row heights, sort label); captures `01-rail-sort` (quota risk, group by project, empty search), Fleet machine filter kept across a reload and shown on the Filters button | PASS |
| Req.3 rows add only exceptions | captures: identity marker, parked "Parked (est.)", "3 uncommitted files", "stale - 3m", desktop "app" chip (a11y and capture suites) | PASS |
| Req.4 Home | vitest `home.test.tsx` (9: needs, words, place, runway measured / from a session / not measured, headline states, Open, no accounts); `67-home` (4 states × 2 sizes × 2 themes) | PASS |
| Req.5 density | vitest (Settings density choice); `01-rail-density` captures for all three | PASS |
| Req.6 session view | vitest `inspector-instance.test.tsx`, `inspector-shell.test.tsx` (summary, Details, plain footers, nothing selected renders nothing); capture "Session Overview ended stamp" | PASS |
| axe | `home-gallery.spec.ts`: 0 critical/serious on every Home state, Filters open and the More menu; the a11y suite unchanged in its checks | PASS |
| Full suite | vitest 450/450; Playwright 897/897 (20 of them new in `home-gallery.spec.ts`) | PASS |

## Fixed on the way

- The More menu was unusable from any screen whose content reached the
  header (only "Triage" could be clicked).
- Hidden button text that started with a space read as "Open— ai-juggler"
  (Home, first run); the space now sits outside the hidden text.

## Not covered here

- At 390 px (below the app's 1024 px minimum, kept as a reflow check) the
  rail stays expanded; Home wraps but is cramped. Collapsing the rail by
  itself at narrow widths is left for the session-view stage.
