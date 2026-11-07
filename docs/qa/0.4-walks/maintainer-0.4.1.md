# The maintainer's 0.4.1 findings — hand walks

- **Build:** 0.4.1 Dev (debug, `dev-bridge,lab-bridge`) at 149747d27 · **Walked by / on:** Claude (side session), 2026-11-06, lab `/tmp/aij-lab-a5` (A5's setup: Work out of room; a lab session on Work and one on Personal) through the lab bridge; both themes, 1024 and 1280.

| Finding | Walked | Seen | Captures (`walks-0.4/maintainer-0.4.1/`) |
|---|---|---|---|
| D-87: the account banner confused which session it was about | Home, the App, the Account, Work's session, Personal's session | No band over any page. Work's session's page: "This session's account, Work, is out of room · 0% of its 5-hour window left · resets at 10:23 PM", its Continue "Continue this session in another app…" opening this session's switch. Personal's session's page: nothing of Work's. The bell and Needs you hold the account's notice; the top bar's count "1" with its gauge, titled "1 account out: open Accounts". | `1-home-*`, `4-session-work-*`, `5-session-personal-*` |
| An unmistakable "Out" on every Account and App that's out | Every place it should show | The rail's App header "Out"; Home's card for the profile with nothing open "Out · resets 10:23 PM" (finding below); the App's and the Account's header chips; New session's app picker lines "Work · Out · resets 10:23 PM"; the switch list's Work Max "Out · resets 10:23 PM", never Suggested (finding below). | `1-home-*`, `2-app-work-*`, `3-account-work-*`, `5-session-personal-*`, `6-picker-*` |
| The header's split button | Its shape and the gap before the avatar | One bordered control, the arrow inside it, a hairline between the halves, a clear gap before the avatar; both themes, 1024 and 1280 (preview). | `7-split-*` |

## Findings on the way (fixed, re-walked the same day)

1. Home's card for a profile with nothing open had no mark (it read its folder from its first session). 149747d27.
2. A switch target on the session's own Account (held back, not ranked "out") said only when it resets. 149747d27.
3. The top bar's count pushed This Mac's line over History at 1280 and wrapped the bar at 1024; the rail's chip cut the profile's name to one letter. c71c77e2d, 22206c46b.

Not walked here: "Go to it opens the exact session" (the live check on aij-test, two sessions, waits on the one-time macOS prompt).
