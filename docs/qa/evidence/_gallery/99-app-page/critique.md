<!-- captures: 35e2e9c44bf7 -->
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

## 0.4, AJ-E3206 req. 2: one mark and one colour per App

Re-shot after each App and each Account got one stored colour. All twelve captures changed. They also catch up with changes made since the last shoot: the Claude app's profile reads "Work Max" (its Code folder's name), the Accounts tab is lit, and a CLI App's header has its ⋯ menu.

**app-overview, app-overview-1024, app-sessions, app-settings (both themes).** Claude app · Work Max's dot beside its name is orange. Before, it was the blue that every "Work Max" thing wore, the CLI App and the Account included. The same orange is on its rail header ("Claude app · W…"), the left edge of its rows, its Needs you rows' corner dots and, in the 1024 captures, the folded rail's icons. Claude Code · Work Max, a different App on the same folder, is magenta in the Needs you row ("Answer the release ques…"). The Limits card's line is the Account's rose now, not the App's colour: the room is the Account's. So the page shows two colours on purpose, the App's dot in the header and its Account's on its limits, and neither is a status colour.

**app-cli (both themes).** Claude Code · Work Max's header dot is magenta, the same as its rail rows and Needs you dot. Its Limits line is the Account's rose. Before, the header was orange and so was the line.

**app-closed (both themes).** Claude app · Lab's dot is green (its stored slot) and matches its tile on Home and its Inactive apps row.

### Weak

- A green App dot right after the app's icon in a header can read as "running" next to "Not running". It's the palette's green, 23° from the success colour, and the page says "Not running" in words.
- The App's dot and its Account's line are two hues on one page, and nothing on it says which is which. The Account's colour is explained only on the Accounts cards ("This account's colour in the charts").
- Magenta (Claude Code · Work Max) and rose (its Account) are 30° apart. They're told apart by shape (a small dot against a line and a round avatar), less by hue.

## 0.4, AJ-E3103: the one related list and the one row

Recaptured: app-sessions, app-overview, app-overview-1024 and app-cli, in both themes. app-closed and app-settings were left as they were.

**app-sessions.** The list opens in sections (Working, Open, Last 7 days). Each row now shows the project's mark and name and when on line 2, with the state word at that line's end. Before, the state led line 2, the project sat in a column and the time in another. With three rows there is no search and there are no chips (they appear past 8).

**app-overview, app-overview-1024, app-cli.** The Open now and Last used cards use the same row. In a card at 1024, the state word stays at the end of line 2 and the place line truncates first.
