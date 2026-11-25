<!-- captures: d506f06e386c -->
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
- every 1280 and 1024 capture with the rail open: the feat/read-model row reads "ai-j…  · 3 h ago", with a wider gap after the "…" than the other rows' " · ".
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

Across the family the darker canvas puts every card a clear step above the page: the Pools cards, Projects' list, a project's four Overview figures and its Activity chart, the Tasks board's cards and the open review panel all read as surfaces, and in light the grey canvas separates the white cards from the page. New session, New pool, New project and the review panel's Approve are one solid violet, so each screen has one obvious primary while New session here, Switch account… and Send back… stay quiet outlines. In dark the lighter accent shows on those buttons, the logo mark, the search field's focus ring (projects-search-empty), the review card's violet frame and "Waits on your review" line, and the ✓ in the Holder menu; the light captures change only by a pixel in the This Mac line. The top bar stays one row: at 1280 with "This Mac · Plenty of room" and "7 to clean up" whole, at 1024 with the list open (header-home, projects) the cleanup pill shortens to a broom and "7", and with the list folded (session-pool-chip) "7 to clean up" fits whole; at 1024 the This Mac line is left out either way. The Holder menu (tasks-holder-menu) is now only as wide as its longest line, so it covers less of the In review column. session-pool-chip-1280x800 is fixed: it now waits for its terminal and shows the "Work Max · session" prompt in both themes, like the 1024 capture, and its rail's feat/read-model row shows "Working" whole with no hover buttons over it. The ink levels work too: the plan's EPIC and STORY labels and the board's breadcrumbs ("Checkout v2 › Totals") sit dimmer than the titles. The second pass changed two things here: in every capture the rail's "Probably asks you" feat/read-model row keeps its time whole ("ai-j… · 3 h ago" where it read "ai-jugg… · 3 …"), with a slightly wide gap after the "…"; and session-pool-chip (both widths, both themes) has the terminal's black filling its card to the edge, without the lighter frame in dark or the white one in light, so the "Work Max · session" prompt sits inside a black margin. What remains: project-pool still sets its times in the monospace face ("2 h ago"), says "1 asks you", and lists four identical "Untitled session · Personal" rows; on the board the Blocked card's "api-gateway ·" ends in a dangling separator.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5)

Opened at full size: header-home-1024x700 light, projects-1280x800 dark, tasks-board-1280x800 light and project-overview-1280x800 dark, beside the 0.3.2 notes.

- **header:** the top bar's current page in bold with no box; at 1024 its words fold to icons (🧹 7, >_) so it stays one row; Home's "Apps & runway ↓" in the head.
- **projects:** "5 projects, the most recently worked first", columns Project · Needs you · Sessions · Tasks · Tokens this week; needs as soft chips ("1 failed", "1 stuck", "1 asks you", "1 at its limit"), "Nothing waiting" muted; New project filled; search by name or folder.
- **project-overview:** storefront's four stat cards (Open sessions 2 · Tasks 3 · New tokens this week 22M ▼53% · Apps used 1) and Activity, new tokens per day by App, Tool or Model.
- **tasks-board:** Ready · In progress · In review · Blocked, Approve filled on the one in review.
- **Weak:** a task card's place line ends on a dangling "·" when its holder or Waits on wraps ("storefront ·", "api-gateway ·"): logged **F63**. docs-site reads "None yet" sessions beside 25M tokens this week (the fixture's, not the product's).

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | Where everything is: the header, Projects, a project's page, Tasks. |
| 2 | Prominence | 4 | Needs you and recent first on Projects; Ready first on Tasks. |
| 3 | Default | 4 | Board by default; Project: All, Pool: All, Holder: Anyone shown with their values. |
| 4 | Mode | 4 | Calm; the project page's stat cards and chart are its rich part. |
| 5 | Mock-up | 4 | Matches the E28 header and Projects specs. |
| 6 | Areas | 4 | Columns and cards by tone. |
| 7 | Colour | 4 | Needs chips in the soft step; the chart's App colours. |
| 8 | Type | 4 | Stat values 28 px, labels 12 px muted; tabular numbers. |
| 9 | Actions | 5 | New project, New session here, Approve: one filled per view. |
| 10 | Serious | n/a | Nothing destructive here. |
| 11 | Doors | 4 | Every project, task and link opens its page; project names on cards are links. |
| 12 | Words | 4 | F63 fixed: a task card's place line is one line, its "·" with who has it. |
| 13 | States | 4 | "Nothing waiting", "None yet", "No open tasks" said once each. |
| 14 | Consistency | 4 | The shared tabs, chips and filters (E28). |
| 15 | Accessibility | 4 | Not re-walked by hand this round; keyboard names and focus as the a11y gate checks them. |
| 16 | Hand walk | 4 | The PL1 walk (AJ-E2323) drove Home and the rail in Dev; not re-walked here. |
| 17 | System prompts | 5 | None. |
| 18 | Usability | 4 | Projects and tasks one click from anywhere. |

Lowest: 4.

**Re-captured after F63 (2026-11-24, Opus):** the family re-rendered; opened tasks-board at 1024x700 light and 1280x800 dark. Each card's place line is one line, the "·" going with who has it or what it waits on: "storefront · Builder · Claude Code CLI · Personal", "api-gateway · Waits on: Refresh-token retries" (they ended on "storefront ·" and "api-gateway ·" over a second line). A card with no place starts with its holder. The chosen segment is F53's solid fill. Lowest stays 4.

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: Needs you's cards are tone fills with their level's mark (a stopped one in the broken tone), the most urgent first (F53, F66, F80). Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.
