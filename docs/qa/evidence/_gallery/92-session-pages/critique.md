<!-- captures: 279b56e9fe47 -->
# 92-session-pages: a session's page, for each kind of session

Design critique (D-57, AJ-E2814) against the reference screens: the row menus (77-menus), the rail's order menu, Usage and Processes. The baseline note says what changed on purpose; this says whether it is good.

## Checklist

| Check | Finding |
|---|---|
| Grid | Header, tabs and the two columns (conversation, Where it runs and Runway) on the page's edges. |
| Hierarchy | Title, then the state chip, then the facts line; Go to it is the one primary. |
| Spacing | This branch: one row of chips, so the header no longer wraps to a second chip row at 1280. |
| Icons | The app's icon with the profile's badge beside it; Go to it carries its icon and ↵. |
| Shared controls | No system controls. |
| Disabled reasons | An ended session offers Continue in AI Juggler and Copy resume command instead of a greyed Go to it. |
| Plain words | "Asks you · 3 min", "Running in the Claude app · viewed 2 min ago", "Started outside AI Juggler: shown here to read and follow". |
| Type sizes | Chips and facts 13 px, body 14 px. |
| Both themes | Checked dark and light. |
| 1280×800 and 1024×700 | Captured at 1280, 1680, 2000 and 2000 at 200 %. |

## Good

- One page for every kind of session: where it runs, its conversation's last turns, Go to it.
- Where it runs says only what it knows (app, window, profile, account, folder).

## Weak

- Before this branch the header repeated itself: "Running in the Claude app", "Profile: Work Max" and the account and folder as chips, while the facts line and Where it runs said them again.

## Fixed

- Re-render: the tool and the pool are named doors on the facts line (the tool was a bare mark).
- Polish (0.3, this branch): the header has two chips, its state and one Account menu (the profile's badge and name ▾: Account, Profile, Folder and Open the profile's page). "Running in …" or "Last active …" is on the facts line after the tool. The separate Where, Profile, Account and Folder chips went; nothing they held is lost (the menu and Where it runs keep it) and no door moved: Go to it, the tool and the pool are where they were.

## Captures

| Capture | What is on it | What changed in 0.3 |
|---|---|---|
| outside-asks-1280x800, -1680x1050, -2000x1100, -2000x1100-zoom200 | "Fix the flaky checkout test", run by the Claude app, asks you: Asks you · 3 min, its 30-turn conversation, the question with Go to it. | 0.3 header. This branch: state chip + Account menu; "Running in the Claude app · viewed 2 min ago" on the facts line. |
| outside-working-1280x800, -1680x1050, -2000x1100, -2000x1100-zoom200 | A Codex thread still working: "Codex is working", its Usage tab. | Same header change. |
| outside-steps-open-1280x800 | A turn's steps opened to their lines; earlier messages loaded. | Same header change. |
| outside-ended-1280x800 | An ended session with no transcript here: Continue in AI Juggler, Copy resume command. | Same; "Last active …" on the facts line. |
| own-overview-1280x800 | AI Juggler's own session on its Overview: its terminal, Where it runs, Runway. | Same; "Running in AI Juggler" on the facts line. |
| last-messages-off-1280x800 | The page with last messages turned off. | Same header change. |
| usage-1280x800, usage-not-measured-1280x800 | The Usage tab with its counts, and with none of its own. | Same header change. |
| before/* | The 0.2 page before the AJ-E2108 redesign, kept for comparison. | Unchanged. |
