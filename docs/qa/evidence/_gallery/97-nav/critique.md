<!-- captures: c73334d9e4e2 -->
# 97-nav: the header, Projects, a project's page, Tasks

Design critique (D-57, AJ-E2814) against the reference screens: the row menus (77-menus), the rail's order menu, Usage and Processes. The baseline note says what changed on purpose; this says whether it is good.

## Checklist

| Check | Finding |
|---|---|
| Grid | The header's seven tabs, This Mac's line and the actions share one row; Projects is a table on the page's columns; the Tasks board is four even columns (2×2 at 1024). |
| Hierarchy | The lit tab is the page's title (heavier, raised); a page below a screen names itself in its breadcrumb. |
| Spacing | Tabs are snug below 1280 so the row fits at 1024 with the list open. |
| Icons | Filter pickers (Project, Pool, Holder) carry their icons; the tabs are words only, as the reference header. |
| Shared controls | Board · Plan is the shared Segmented; the filters are the shared Picker. |
| Disabled reasons | The plan has no Holder filter (its epics and stories have no holder) rather than a dead one. |
| Plain words | "Waits on your review", "No one yet", "Nothing waiting". |
| Type sizes | Tabs 13 px, body 14 px; nothing under 12 px. |
| Both themes | Checked dark and light. |
| 1280×800 and 1024×700 | One header row at both; the board folds to 2×2 at 1024. |

## Good

- The header is the strongest change: every door visible, the lit tab is the page's title, one row at both widths.
- A project's page: breadcrumb, folder and branch, tabs with counts, Overview's four figures and the Activity chart on one grid.
- The Tasks board's four even columns keep full titles and the review card's call to action.

## Weak

- Projects rows at 1280 leave a wide empty middle; the list is one line a project.
- The board's cards are text only (no mark for the holder's tool); "Waits on your review: open it to approve or send back" is long for a card.
- The plan's EPIC and STORY in capitals beside a pill read busy.
- tasks-board, tasks-holder-menu, tasks-open-review: the Blocked card "Rotate signing keys" ends its line with a dangling separator ("api-gateway ·") where a holder would go.
- project-pool-1280x800: the header chip and facts line say "1 asks you"; the four rows are all "Untitled session · Personal", told apart only by state and time, and the times ("2 h ago") are in the monospace face where every other list uses the body face.

## Fixed

- Re-render: a project's Pool tab names members by the session's title (it said "its folder"); its Subagents tab hides the dead toolbar when there are none; a paired Mac's session page lights Fleet; Fleet's tab no longer wraps the bar to two rows.
- Re-render: a session's tool and pool are named doors on its facts line ("Claude Code · … · Pool"), so the chips no longer cut to "Running in AI J…".
- Polish (0.3, this branch): History has a key, ⌘⇧Y, in the same letter scheme as ⌘⇧P, ⌘⇧O, ⌘⇧T, ⌘⇧A and ⌘⇧U (⌘⇧H stays Continue on another account); every header tab's tooltip names its key.
- Polish (0.3, this branch): a session's header has two chips, its state and one Account menu (the profile's badge and name ▾: Account, Profile, Folder and Open the profile's page). "Running in …" or "Last active …" moved onto the facts line after the tool; the separate Where, Profile, Account and Folder chips went. Go to it, the tool and the pool doors are unchanged.

- Second re-render: the captures were looked at again after the rail's shorter top and the session header's Account chip; the header row stays one row at 1024 with the list folded, the board stays 2×2, and the session header is two chips (Working, Work Max ▾) with Claude Code, Running in AI Juggler, started, CPU and Pool on the facts line. Nothing else to fix here.

## Captures

| Capture | What is on it | What changed in 0.3 |
|---|---|---|
| header-home-1280x800 | Home lit: Home · Projects · Pools · Tasks · Accounts · Usage · History, This Mac's line, 7 to clean up, New session. | New header (AJ-E2803). This branch: tab tooltips with keys. |
| header-home-1024x700 | The same row at 1024 with the list opened. | New. |
| projects-1280x800, -1024x700 | Five projects, the most recently worked first: what needs you (1 failed, 1 stuck…), sessions, tasks ("1 ready · 1 in review") and this week's tokens. | New screen. |
| projects-search-empty-1280x800 | A search that finds nothing, with how to clear it. | New. |
| project-overview, -tasks, -pool, -subagents, -history (1280x800) | A project's five tabs: four figures and Activity; its pool's tasks; its pool in brief; "None of these sessions has started a subagent"; its past sessions. | New page; re-render fixes on Pool and Subagents. |
| tasks-board-1280x800, -1024x700 | Seven open tasks in Ready 3 · In progress 2 · In review 1 · Blocked 1. | New screen. |
| tasks-open-review-1280x800 | Checkout copy pass open beside the board with Approve and Send back. | New. |
| tasks-plan-1280x800 | The plan: epics, stories and their tasks behind folds. | New. |
| tasks-holder-menu-1280x800 | The Holder picker open on the board. | New. |
| pools-1280x800 | The Pools screen's cards. | New. |
| session-pool-chip-1280x800, -1024x700 | feat/read-model's header: its chips, the facts line with Claude Code and Pool as doors. | Re-render fix. This branch: state chip and Account menu only; where it runs on the facts line. |

## 0.3.2

Across the family the darker canvas puts every card a clear step above the page: the Pools cards, Projects' list, a project's four Overview figures and its Activity chart, the Tasks board's cards and the open review panel all read as surfaces, and in light the grey canvas separates the white cards from the page. New session, New pool, New project and the review panel's Approve are one solid violet, so each screen has one obvious primary while New session here, Switch account… and Send back… stay quiet outlines. In dark the lighter accent shows on those buttons, the logo mark, the search field's focus ring (projects-search-empty), the review card's violet frame and "Waits on your review" line, and the ✓ in the Holder menu; the light captures change only by a pixel in the This Mac line. The top bar stays one row: at 1280 with "This Mac · Plenty of room" and "7 to clean up" whole, at 1024 with the list open (header-home, projects) the cleanup pill shortens to a broom and "7", and with the list folded (session-pool-chip) "7 to clean up" fits whole; at 1024 the This Mac line is left out either way. The Holder menu (tasks-holder-menu) is now only as wide as its longest line, so it covers less of the In review column. session-pool-chip-1280x800 is fixed: it now waits for its terminal and shows the "Work Max · session" prompt in both themes, like the 1024 capture, and its rail's feat/read-model row shows "Working" whole with no hover buttons over it. The ink levels work too: the plan's EPIC and STORY labels and the board's breadcrumbs ("Checkout v2 › Totals") sit dimmer than the titles. What remains: project-pool still sets its times in the monospace face ("2 h ago"), says "1 asks you", and lists four identical "Untitled session · Personal" rows; on the board the Blocked card's "api-gateway ·" ends in a dangling separator.
