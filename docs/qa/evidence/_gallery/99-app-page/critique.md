<!-- captures: 77e1461a41c6 -->
# 99-app-page: an App's own page (0.4, D-62, AJ-E3204)

Design critique (D-57, AJ-E2814). First critique of this family, written at 0.4 from its captures.

## 0.4

The audit's G1/G2: clicking "Claude app · b-work" in the rail opened a folder's page titled "Account" that also listed Terminal sessions; nothing answered "this app: its sessions, projects, settings". Five screens, both themes, on the places desktop (two Claude app profiles and ChatGPT running, a Lab profile kept alive that's down), each opened the way a user would: from the rail's App header, or from a Home tile's app name.

**app-overview.** "App" over the name, the Claude app icon with Work's profile dot beside it (D-38), and "Claude app · Work" in the page title size. One facts line: "Claude app · Running · kept alive · 3 sessions · 2 open". One primary button at the right, "Go to it". Tabs with counts: Overview, Sessions 3, Projects 2, Usage, Processes (it has open sessions), Settings. Two cards side by side: "Open now 2 open" with its two sessions as the shared session rows, and "Last used" with the one that ended. Below, its account's Limits card (the folder's runway and the last 7 days of tokens), then "Projects, where it works, most recent first", each with its session count and when. The rows don't repeat "in Claude · Work Max" or the account: the page says both once. At 1024 the two cards stay side by side and the rows cut their second line with an ellipsis.

**app-sessions.** "3 sessions · 2 open" then the paged list, open first: title, state and branch, the project column, when. No account column (the same on every row here).

**app-settings.** Two sections drawn as Settings › Desktop apps draws them: Name ("Claude app · Work", "The name AI Juggler gives it: its app, then its profile.", Rename), and Start and keep alive (the same row as Settings: Open, and the Keep alive switch on).

**app-closed.** Lab, down and kept alive: "Not running · keep alive starts it · No sessions yet", the primary button is "Start", and the body is one card, "No sessions yet", saying what will show there and what to do first, instead of three empty cards. No Usage or Processes tab: nothing to show.

**app-cli.** "Claude Code · Work Max", "Claude Code in a terminal · 3 sessions · 3 open", primary "New session". Its background import run is under Open now (it's running), so "Last used" says "Everything here is open now."; its folder's Limits and its one project follow.

## Fixed while critiquing

- The Settings rows were list items outside a list (axe: listitem); they're rows of a section now, with the Settings card around them.
- Every row said "in Claude · Work Max" and the account, and an ended session said "in Claude Code" on the Claude app's page; rows on an App's page leave both out.
- A running background session sat under "Last used"; it's under Open now.
- An App with no sessions showed three empty cards ("No other sessions here yet."; "its sessions so far ran outside one"); now one card says what to do.
- Settings › Desktop apps named a profile "Claude · Work Max" (its account) while the rail said "Claude app · Work"; both say the App's name now.

## Weak

- The facts line doesn't name the Account yet ("on Work Max", a door to the Account page): that comes with AJ-E3205 and AJ-E3206.
- Limits on a desktop App show its folder's, and say "Not measured yet" where the folder has no reading; whose room it is should be said in the card's title once the Account page exists.
- Processes lists its open sessions' processes, not the desktop app's own helper processes.
- Rename is in Settings only; the header's ⋯ menu (rename, hide from the rail, show the folder) is still to come.
