# AJ-E3510 doors walked on the release build (2026-11-06)

AI Juggler Dev, the release build (`scripts/floor_harness.py build`, at
c6e4a92e8's code), in a lab HOME made by `scripts/qa/lab_scenario.sh setup
P2` (generic: one App "Claude Code · Work · work" on the Account "Work",
`w…@example.invalid`, a pool, four ended sessions), `AIJ_NO_KEYCHAIN=1`,
watched for a keychain dialog (none came). The Floor window and the main
window side by side; driven by synthetic mouse events to that pid only.
Times run from the click to the main window in front; they include the
walk tool's own pauses (about 0.5 s).

| Door | Hover says | A click opens | Time | Capture |
|---|---|---|---|---|
| The header's Floor button (main window) | — | the Floor, Floor 1 | 696 ms to the window | — |
| The App's area | "CLAUDE CODE · WORK · WORK / 0 SESSIONS · WORKSHOP", the hand | the App's page, "Claude Code · Work · work" | 618 ms | `dev-hover-area-*`, `dev-door-area-*` |
| This Mac's rack | "THIS MAC / LOAD AND MEMORY", the hand | the This Mac page | 617 ms | `dev-hover-rack-*` (the page shows this Mac's own apps: not committed) |
| The notice board | "NOTIFICATIONS / 7 UNREAD", the hand | the notification centre, open | 602 ms | `dev-hover-notice-*` (the list names this Mac's own profiles: not committed) |
| The whiteboard | "… · TASKS / 0 TASKS", the hand | Tasks, the board, Holder: "Claude Code · Work · work" | 614 ms | `dev-hover-board-*`, `dev-door-board-*` |
| The usage board | "WORK / USAGE LEFT: NOT KNOWN", the hand | the Account "Work"'s page | 602 ms | `dev-hover-usage-*`, `dev-door-usage-*` |
| The lift | "FLOOR 1 / THE LIFT: GO TO ANOTHER FLOOR", the hand | stays on Floor 1 (the only floor) | — | `dev-hover-lift-*` |

## Found and fixed

- The App's name read "CLAUDE CODE · WORK · WORK" on the Floor: a profile
  named as its Account in another case. Fixed in c6e4a92e8 (said once);
  this build was from before the fix.

## Not walked here, and why

- A person (the session, at its question or error), its helpers, and the
  pool's meeting room: all need a live session, and a live session needs a
  tool signed in; the lab's seeded sessions end when the app starts (no
  process runs them). Their doors are the same path (`door()` → `OpenRequest`
  → `routeOpen`), tested in `each_door_opens_its_page_and_toys_open_nothing`,
  `outside.test.ts` and the Floor's scene tests. They are walked with the
  persona walks (AJ-E3514) on the release candidate's lab with a live tool.
