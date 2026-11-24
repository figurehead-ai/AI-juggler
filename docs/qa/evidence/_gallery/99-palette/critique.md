<!-- captures: 8a7ebb106936 -->
<!-- captures: pending -->
# 99-palette: ⌘K — search, act or go

Design critique (D-57) against the reference screens: the row menus (77-menus), the rail's search and Settings' search. New family (AJ-E2901).

## Checklist

| Check | Finding |
|---|---|
| Grid | One dialog, field on top, groups under their labels, the footer in two lines (keys, then See all results and the go-to key); the preview takes a column on the right. |
| Hierarchy | Group labels small and muted; each row's name, then where in muted words, then what choosing it does at the end. |
| Spacing | The menu rhythm; the dialog widens from 640 to 920 px only while the preview shows. |
| Icons | Each row its own mark: the app's icon, the project's mark, the command's icon (the same as elsewhere). |
| Shared controls | The shared Button in the preview and the footer; the combobox and listbox pattern with groups. |
| Disabled reasons | A selected item's greyed action keeps its reason (from the registry). |
| Plain words | "Processes — 7 left running", "Notifications — 1 unread", "No matches for “qqqzz”.", "AI Juggler's service isn't answering — showing what was open last." |
| Type sizes | 13 px rows, 12 px labels and footer. |
| Both themes | Checked; the matched letters are bold (not only coloured) in both. |
| 1280×800, 1024×700 and 1440×900 | All three; the preview fits beside the list at 1024, and at 1440 the dialog keeps its width rather than stretching. |

## Good

- Before typing it is useful: what you opened last, then the four things you do most, then what's open.
- The matched letters are bold ("pay" in payments-api), and a row named by what's typed starts active, so ↩ opens it.
- No matches leaves Search History for the words active, one ↩ away.

## Weak

- A loose letter match ("pay" in Privacy) still shows in Actions above the project it isn't; it is no longer the active row, but it reads as noise.
- The footer's go-to key button reads long ("Choose a key to open this from any app…") beside See all results.

## Fixed

- First render (seen by eye): typing "pay" left "Go to Privacy" active, so ↩ went to Privacy, not payments-api. The first row whose name starts with what's typed now starts active (vitest: "a row named by what's typed starts active…").
- First render: the footer's key hints wrapped mid-phrase on one line with the buttons; the hints have their own line now.

## 0.4.1 re-score (2026-11-07, AJ-E3305)

The whole family was retaken on release/0.4.1 for the 14-line re-score (AJ-E3305, `docs/qa/0.4-walks/design.md`) and opened on contact sheets, both themes, 1280 and 1024. Shared with every screen since the last shoot: the header's New session is tinted (one filled button per view), the Floor button is a split control with its own menu, This Mac's line is its dot below 1120 px, and the rail's App headers put the profile first.

Unchanged in design: the exact match first ("payments-api" over Go to Privacy), the preview with Open as its one filled button.

## 0.5.1: Search takes over Bot's look-ups (AJ-E3824, D-144)

Retaken: empty, records (new), no matches and offline, both themes, 1280, 1024 and 1440; opened by eye in both themes at 1280.

- **Empty:** Actions gains "Watch a session…" (the Watches screen's bell, as elsewhere); a Suggested group (What needs me, What finished today, Runway by account, What’s running) sits between Actions and Open now, the same 13 px option rows as every group. The field's hint names what Search finds: "Search sessions, tasks, pools, usage, apps and projects…".
- **Records** ("What needs me", picked from Suggested): Sessions, Tasks, Pools and Usage in the palette's own row pattern: the mark (the session's app, or the screen's icon), the name, then where and how it stands in muted words ("Release · builder-2 · Blocked", "this week · 54 % used"), and "↩ Open" on the active row only. Each record shows once: a session already listed under Sessions is not repeated.
- **No matches:** "No matches for “qqqzz”." with a second, smaller line saying where it looked; Search History stays the active row; Suggested follows.
- **Weak:** the Suggested entries use the History clock (the "search" icon the palette already maps); a magnifier would read better, but none is in the icon set yet. With many recents and actions, Open now starts lower than before (one more group above it).

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5)

Opened at full size: empty (1280 dark), records (1280 light), no-matches (1024 light) and preview (1280 dark); typed and offline only as the AJ-E3824 notes above describe them. Set beside the calm reference (`AIJuggler-design-refs/calm-ui`: the Claude app's Code view and the round-2 mock), the palette is as quiet: one surface on the overlay tone, group labels in the section tone, one accent (the active row's name and the field's focus line), muted where-words, keys right-aligned. Since the AJ-E3824 notes above: F44 ends the list on a whole row, so at 1280 Suggested shows whole above the footer (empty), and at 1024 no-matches keeps Search History active with all four Suggested entries.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 5 | Find, act or go from one field; records (What needs me) answer in place. |
| 2 | Prominence | 4 | Recent, then Actions, then Suggested: the common path first; Open now starts lower with Suggested above it (noted before). |
| 3 | Default | 5 | The best match starts active; ↩ opens it. |
| 4 | Mode | 5 | Calm: one overlay surface, no cards. |
| 5 | Mock-up | 4 | No mock of its own; matches the menus and the calm kit. |
| 6 | Areas | 5 | Field, list, footer; the preview a column. |
| 7 | Colour | 4 | Accent only on the active row and focus; "Suggested" uses the History clock (no magnifier in the set). |
| 8 | Type | 4 | 13 px rows, 12 px labels and keys; where-words muted. |
| 9 | Actions | 5 | Open is the preview's one filled button. |
| 10 | Serious | n/a | Nothing destructive here. |
| 11 | Doors | 5 | Every row opens its thing; See all results goes to History. |
| 12 | Words | 4 | "Looked in sessions, tasks, pools, usage, apps, projects, accounts and settings."; the footer's "Choose a key to open this from any app…" still long. |
| 13 | States | 5 | Empty, no matches, offline ("showing what was open last") each said once. |
| 14 | Consistency | 4 | The field's focus line sits 3 px from the typed text (no inner padding), where Settings' search has room; left as is because 77-menus and 95-app-icons show the palette too. |
| 15 | Accessibility | 4 | Combobox and listbox with groups; focus stays in the field (a11y.spec). Not re-walked this round. |
| 16 | Hand walk | 4 | PL1 walk (AJ-E2323) drove ⌘K with F25's chords; not re-walked here. |
| 17 | System prompts | 5 | None. |
| 18 | Usability | 5 | AJ-E3824's answers hold (below). |

**AJ-E3824 match:** (1) one search for everything: the hint names sessions, tasks, pools, usage, apps and projects. Match. (2) Empty query shows the four Suggested entries and Watch a session…, plainly named. Match. (3) No second search: Bot has none (D-144). Match. (4) No results says where it looked and keeps Suggested; no setup state. Match. (5) None. Match. (6) Each record once (a session under Sessions isn't repeated). Match.

Lowest: 4 (lines 2, 5, 7, 8, 12, 14, 15, 16). No fix made; nothing logged.
