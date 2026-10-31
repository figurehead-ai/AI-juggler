<!-- captures: 7f54a59daca1 -->
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

- The Overview's footnote ("from the apps' own session files and the end of its transcript") is a lowercase fragment under the page; it reads as a sentence cut off. It is the app-wide "from …" source pattern, kept as it is.
- The Account menu's first line is the plan's name ("Claude Max") under the heading Account, while the chip says the profile ("Work Max"); clear once open, but the chip and its menu lead with different names.
- outside-working-1280x800 (dark): the "Continue on another account" card ends under the "More below" fade, which slices "Week resets Thu 2:13 AM" in half.
- outside-asks-2000x1100-zoom200 (both themes): the conversation shrinks to the cut bottom of one line ("merge sort it out?") above the asks band.
- usage-1280x800, usage-not-measured-1280x800: the chart legend (Input 5.3M, Output 1.2M; 1.3M, 282k) and By kind (6.2M, 1.4M; 1.4M, 301k) give different totals for the same session without saying why.
- outside-steps-open-1280x800 (both themes): "Ran 2 commands" is still folded, so the capture named for opened steps shows none.
- outside-steps-open-1280x800-dark: the rail is scrolled so its search, the App · Project · Account switch and the Needs you header are gone, unlike the light capture and every sibling.
- outside-asks, outside-steps-open, last-messages-off, account-menu-open: two solid violet Go to it buttons on one screen (header and the "Claude asks you" band).

## Fixed

- Re-render: the tool and the pool are named doors on the facts line (the tool was a bare mark).
- Polish (0.3, this branch): the header has two chips, its state and one Account menu (the profile's badge and name ▾: Account, Profile, Folder and Open the profile's page). "Running in …" or "Last active …" is on the facts line after the tool. The separate Where, Profile, Account and Folder chips went; nothing they held is lost (the menu and Where it runs keep it) and no door moved: Go to it, the tool and the pool are where they were.

- Second re-render: looked at again by eye at 1280, 1680 and 2000 in both themes: one chip row, the Account menu opens under its chip without covering the title, the facts line carries where it runs. Nothing else to fix here.

## Captures

| Capture | What is on it | What changed in 0.3 |
|---|---|---|
| outside-asks-1280x800, -1680x1050, -2000x1100, -2000x1100-zoom200 | "Fix the flaky checkout test", run by the Claude app, asks you: Asks you · 3 min, its 30-turn conversation, the question with Go to it. | 0.3 header. This branch: state chip + Account menu; "Running in the Claude app · viewed 2 min ago" on the facts line. |
| outside-working-1280x800, -1680x1050, -2000x1100, -2000x1100-zoom200 | A Codex thread still working: "Codex is working", its Usage tab. | Same header change. |
| outside-steps-open-1280x800 | A turn's steps opened to their lines; earlier messages loaded. | Same header change. |
| outside-ended-1280x800 | "Move the orders API to v2", ended, no transcript here: Ended and Personal ▾ under the name, "Claude Code · Last active 5 h ago" under them, Continue in AI Juggler and Copy resume command. | Same; "Last active …" on the facts line. The spec opens the project's Sessions tab (a project opens on Overview since 0.3; the state had stopped running). |
| account-menu-open-1280x800, -1024x700 | The Account chip (Work Max ▾) opened on "Fix the flaky checkout test": Account over Claude Max · o…@work.com · Max, Profile Work Max, Folder ~/Dev/web-app, then Open Work Max's page, focused. | New state (this branch). |
| own-overview-1280x800 | AI Juggler's own session on its Overview: its terminal, Where it runs, Runway. | Same; "Running in AI Juggler" on the facts line. |
| last-messages-off-1280x800 | The page with last messages turned off. | Same header change. |
| usage-1280x800, usage-not-measured-1280x800 | The Usage tab with its counts, and with none of its own. | Same header change. |
| before/* | The 0.2 page before the AJ-E2108 redesign, kept for comparison. | Unchanged. |

## 0.3.2

Go to it is solid violet in the header, and the "Claude asks you" band below carries a second solid violet Go to it, so outside-asks, outside-steps-open, last-messages-off and account-menu-open show two equal primaries on one screen. Cards stand off the canvas in both themes, the band is a warm amber (dark) or peach (light) well, and outside-ended's "Continue in AI Juggler" is the single primary beside an outlined Copy resume command. After the design fixes the dark captures carry the lighter accent on the logo mark, both Go to it buttons, New session, Continue in AI Juggler, the Runway bars, the Usage tab's bar charts and the "Measure its limits ›" link, all brighter against the near-black; the light captures changed only in the top bar, which stays one row: at 1280 and up "This Mac · Plenty of room" and the whole "7 to clean up" pill, at 1024 (account-menu-open-1024x700) a broom and "7" in place of the words, so the bar no longer crowds Go to it. The right column's "Continue on another account" card still ends under the "More below" fade, which in outside-working-1280x800 slices "Week resets Thu 2:13 AM" in half, and at 200 % zoom (outside-asks-2000x1100-zoom200) the conversation still shrinks to the cut bottom of one line above the asks band. On the Usage tab the chart legend and the By kind list give different totals for the same session (Input 5.3M and Output 1.2M against 6.2M and 1.4M; 1.3M and 282k against 1.4M and 301k in usage-not-measured) without saying why. outside-steps-open shows "Ran 2 commands" folded rather than its steps, and its dark capture now has the rail scrolled to put the selected "Fix the flaky checkout test" row at the very top, with the search, the switch and the Needs you header out of view. The rail's "ai-jugg… · 3 …" row still cuts its age. The sixteen before/* captures were not re-rendered and keep the 0.3.1 colours.
