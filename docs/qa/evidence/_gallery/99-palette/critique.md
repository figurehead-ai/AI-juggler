<!-- captures: 54ea63300899 -->
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
