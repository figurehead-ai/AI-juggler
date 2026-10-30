<!-- captures: none -->
# 97-controls: the shared controls and the rail's View

Design critique (D-57, AJ-E2814) against the reference screens: the row menus (77-menus), the rail's order menu, Usage and Processes. The baseline note says what changed on purpose; this says whether it is good.

## Checklist

| Check | Finding |
|---|---|
| Grid | Every line is the shared choice line: icon column, label, ✓ column; Only's values sit in the ✓ column with a chevron. |
| Hierarchy | "View" heading, then Show and Only as small section names, never lines you pick. |
| Spacing | This branch: every View line is one height. |
| Icons | Every Show and Only line has its icon, in the app's stroke style. |
| Shared controls | No system select, radio or checkbox: the View is a menu of choice lines; Settings › General uses the shared radio cards; History's filters are the shared Picker. |
| Disabled reasons | Reset shows only when something changed instead of sitting greyed. |
| Plain words | "Quiet ones", "Background runs", "Other Macs"; hints say what each is. |
| Type sizes | Labels 14 px, hints and section names 12 px. |
| Both themes | Checked dark and light; the grouping switch's chosen segment is faint in light. |
| 1280×800 and 1024×700 | At 1024 the list is opened first (the folded column's Group and View door). |

## Good

- The View is a real menu: icon · label · hint · ✓ on one grid, Only as submenus with ✓ on the chosen value.
- Settings › General's equal cards and shared radios.

## Weak

- Hints sat on some lines and not others (Inactive had one, Ended today, Other Macs and Archived didn't), so rows had two heights (fixed in this branch).
- The menu covers the grouping switch beside it while open.
- History's Picker open wasn't captured (fixed in this branch).

## Fixed

- Re-render: the grouping switch's segments size by their words (Pool cut "Account" to "Accou…").
- Polish (0.3, this branch): the View's rows are one height; a hint sits on the label's line, muted, in shorter words, only where the label needs it.
- Polish (0.3, this branch): History's Picker open is captured (the fixture always had three projects; the state counted the filters before the lazily loaded screen drew, and skipped).

## Captures

| Capture | What is on it | What changed in 0.3 |
|---|---|---|
| rail-group-switch-1280x800, -1024x700 | The rail's App · Project · Account · Pool switch with Project chosen (a pool of yours exists). | New (AJ-E2802). |
| view-open-1280x800, -1024x700 | The View as it opens: Show's seven lines, Only's Account and Project, no Reset. | New menu (AJ-E2801, AJ-E2802). This branch: one row height. |
| view-changed-only-open-1280x800, -1024x700 | Quiet ones turned on, Reset shown, Only › Account's submenu open with ✓ on All. | New. This branch: one row height. |
| picker-open-history-1280x800, -1024x700 | History with its Project filter open: "Project" over All ✓ (focused), ai-juggler, storefront, sacrificial, under the Project: All and Profile: All chips. | New state (this branch): the spec counted the filters before History had drawn and skipped. |
| settings-general-cards-1280x800, -1024x700 | Settings › General: theme, density and order as equal cards with the shared radio. | New (AJ-E2813). |
