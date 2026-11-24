<!-- captures: 0bf685a1db61 -->
# 99-inactive-apps: the rail grouped by App is about apps (0.3.4, D-64)

Design critique (D-57, AJ-E2814). First critique of this family, written at 0.3.4 from its captures.

## 0.3.4

The maintainer's report: a Claude app that quit overnight vanished from the rail with no way to start it, and "23 inactive" listed sessions from all over the place. Four screens, both themes, on the places desktop (two Claude profiles and ChatGPT running, a Lab profile kept alive that's down, an old one), plus a CLI app with nothing open: Claude Code on a "Side project" folder with two sessions that ended yesterday and two days ago.

**rail-inactive-apps.** Under Open now, each app with sessions that aren't open ends with a quiet link in the accent colour, "Last 1 inactive", with a chevron at the right like the section heads'. After the last open app comes a section head, "Inactive apps 3", with a clock icon and the secondary-text colour, so it sits below Open now's green in weight. Its rows are drawn like place headers: the app icon, the app name in secondary text (dimmer than an open app's), the profile badge and name, then a second line, "Not running · 20 min ago" or "20 h ago · 2 sessions". The rows are in most-recently-used order. Each has one 24 px action at the right, in the same outline as an open app's ↗: ▷ Start for a desktop app that's down, + New session for a CLI app. A row with sessions has the fold chevron at the left; one without keeps the space, so the icons line up. "Also:" no longer lists inactive sessions: "1 ended today · 1 background · History ›".

**rail-last-3.** "Last 1 inactive" unfolded under Claude app · Personal shows its ended session as a normal row ("Ended") under the link, its chevron turned down. The Side project app unfolded shows its two sessions with the folder's colour edge, newest first.

**rail-idle-menu.** A right-click on Claude app · Lab opens its menu: the name as the heading, Start, then Keep alive with the violet ✓, as the place header's menu draws it.

**home-closed-apps.** Your apps: the four open tiles, then Lab and Old as dashed tiles with "Start Claude app".

## Weak

- A row's second line can still be cut at the narrowest rail width when the profile name is long. The full words are its accessible name, but there's no tooltip.
- An inactive app with more than 3 sessions shows only its last 3; the rest are in History until the App page (0.4) lists them.
- The menus have no icons, matching the place header's menu. Icons for both menus come with 0.4's App page actions.

## 0.4, AJ-E3206 req. 2: one mark and one colour per App

Re-shot after each App and each Account got one stored colour. All eight captures changed.

**rail-inactive-apps, rail-last-3, rail-idle-menu (both themes).** Each App's dot is now the same in Open now, Inactive apps and Home. Claude app · Work is orange on its header, edge and tile (before, blue). Claude Code · Work Max is magenta (before, orange: the same orange the Account had). Lab is green, Old violet and Side project blue. The Side project's sessions keep its blue edge when unfolded.

**home-closed-apps (both themes).** Your apps' tiles have the rail's colours: Personal pink, Work Max orange, Claude Code · Work Max magenta, and the remembered Lab green and Old violet. "Runway by account" shows each Account's round mark in its own colour, rose and teal. Before, they were orange and violet, the colours of their first folders. The Needs you rows' corner dots are their Apps' colours.

### Weak

- Violet (Old) and magenta (Claude Code · Work Max) are 20° apart. At the rail's 6 px dot they're hard to tell apart, though the name beside each dot is the real label.

## 0.4.1 re-score (2026-11-07, AJ-E3305)

The whole family was retaken on release/0.4.1 for the 14-line re-score (AJ-E3305, `docs/qa/0.4-walks/design.md`) and opened on contact sheets, both themes, 1280 and 1024. Shared with every screen since the last shoot: the header's New session is tinted (one filled button per view), the Floor button is a split control with its own menu, This Mac's line is its dot below 1120 px, and the rail's App headers put the profile first.

Unchanged in design: Inactive apps folded under Open now with last-seen times, Home's closed tiles with Start.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5)

Opened at full size: rail-idle-menu (dark) and rail-inactive-apps (light), beside the 0.4.1 re-score. The App lens groups by App; inactive Apps sit under "Inactive apps 4" with their last use ("Claude app · Lab · Desktop · 20 min ago") and a ▷ Start (+ for a terminal App's new session); the row's menu offers Start, Keep alive ✓ and Open its page. Home's Your apps lists running ones, then "Not running" with Start, and "kept alive"/"Closed · keep alive starts it" said in the account line.

- **Since 0.4.1:** calm rail and Home; the heading counts "2 need you · 7 open · 1 to review · 1 inactive session"; F22's head.
- **Weak:** the idle App's menu lines carry no icon (Start, Keep alive, Open its page; N8): logged **F60** with the feed's ⋯. Home's empty feed shows its chips (F56).

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | See every App, running or not, and start one from the rail or Home. |
| 2 | Prominence | 4 | Running first, inactive folded under its count. |
| 3 | Default | 4 | Keep alive's state shown with ✓; Start the row's own button. |
| 4 | Mode | 4 | Calm. |
| 5 | Mock-up | 4 | Matches calm-ui's inactive-apps shots (rail and Home). |
| 6 | Areas | 4 | Groups by tone; the menu on the overlay tone. |
| 7 | Colour | 4 | App hues once per row; amber only on needs. |
| 8 | Type | 4 | App names 13 px semibold, facts 12 px muted. |
| 9 | Actions | 5 | Open its page ↵ on Home the one filled; Start tinted. |
| 10 | Serious | n/a | Nothing destructive (Remove is in the App's page). |
| 11 | Doors | 4 | Open its page from the menu; each row opens its App. |
| 12 | Words | 4 | "Closed · keep alive starts it": says what will happen. |
| 13 | States | 4 | Four inactive, real-sized. |
| 14 | Consistency | 4 | F60 fixed: the idle App's menu has the rail menus' icons. |
| 15 | Accessibility | 4 | Not re-walked by hand this round; keyboard names and focus as the a11y gate checks them. |
| 16 | Hand walk | 4 | The PL1 walk (AJ-E2323) drove Home and the rail in Dev; not re-walked here. |
| 17 | System prompts | 5 | None from the rail (Start launches the user's own App). |
| 18 | Usability | 4 | An App not running is one click from starting; nothing is added by itself. |

Lowest as first scored: 3 (line 14, F60 logged: icons need meanings chosen in the icon set).

**Re-captured after F60 and F56 (2026-11-24, Opus):** all eight re-rendered; opened rail-idle-menu in both themes and home-closed-apps. The idle App's menu: Start (its row's ▷), Keep alive (a shield, its ✓ at the end), Open its page (a page): each line with its icon, as the rail's other menus (they were words alone). An open App's header menu got the same. Home's empty feed is its card alone, with Include my actions in it (F56).

Lowest: 4 (line 14 was 3: F60 fixed).

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: The system-prompt card is restyled slightly (tone, no bright border). Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.
