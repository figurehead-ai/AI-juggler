<!-- captures: 9331f0852120 -->
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
- Second re-render, seen by eye on picker-open-history: History's rows stopped with an error said "7 h" and "12 h" beside their neighbours' "5 h ago" (fixed, below).
- settings-general-cards-1024x700: the three order cards squeeze their hints to one or two words a line ("Stays put. / New ones / join at the / end.") and "Reset my order" wraps onto two lines inside its pill.
- view-open, view-changed-only-open (both widths): the page heading "5 sessions need you" peeks out half-covered above the View menu's top edge.

## Fixed

- Re-render: the grouping switch's segments size by their words (Pool cut "Account" to "Accou…").
- Polish (0.3, this branch): the View's rows are one height; a hint sits on the label's line, muted, in shorter words, only where the label needs it.
- Polish (0.3, this branch): History's Picker open is captured (the fixture always had three projects; the state counted the filters before the lazily loaded screen drew, and skipped).

- Second re-render (seen by eye): a session stopped with an error has ended, so its row says "7 h ago" like every other ended row (History, a project's Sessions); it was timed like a live one ("7 h"). Vitest: "says \"ago\" for every ended session, one stopped with an error too".

## Captures

| Capture | What is on it | What changed in 0.3 |
|---|---|---|
| rail-group-switch-1280x800, -1024x700 | The rail's App · Project · Account · Pool switch with Project chosen (a pool of yours exists). | New (AJ-E2802). |
| view-open-1280x800, -1024x700 | The View as it opens: Show's seven lines, Only's Account and Project, no Reset. | New menu (AJ-E2801, AJ-E2802). This branch: one row height. |
| view-changed-only-open-1280x800, -1024x700 | Quiet ones turned on, Reset shown, Only › Account's submenu open with ✓ on All. | New. This branch: one row height. |
| picker-open-history-1280x800, -1024x700 | History with its Project filter open: "Project" over All ✓ (focused), ai-juggler, storefront, sacrificial, under the Project: All and Profile: All chips. | New state (this branch): the spec counted the filters before History had drawn and skipped. |
| settings-general-cards-1280x800, -1024x700 | Settings › General: theme, density and order as equal cards with the shared radio. | New (AJ-E2813). |

## 0.3.2

The View menu (view-open, view-changed-only-open) and History's Project picker (picker-open-history) float on a lifted surface over the darker canvas, and in light the white menus separate from the grey page. The picker changed most this round: it now opens right under its "Project: All" chip, only as wide as its longest name, with "All" ✓ on top, then ai-juggler, storefront and sacrificial (it used to hang wider and to the left of the chip). In dark the lighter accent shows on the header's New session, the logo mark, the ✓ marks, the Runway bar and Measure its limits; light changes only by a pixel in the This Mac line. The top bar stays one row at both widths: at 1280 with "This Mac · Plenty of room" and "7 to clean up", at 1024 with the cleanup pill shortened to a broom and "7" and the This Mac line left out. rail-group-switch shows the rail age fix: the Project rail's "Stopped with an error" and "Limit reached · 0% left" now end whole instead of on a cut-off "· 7…", while rows with room still show their age ("Asks you · 2 h", "Probably stuck · 3 h"). In settings-general-cards the chosen Light or Dark, Comfortable and Your order cards are a clearly tinted, outlined step above their neighbours in both themes, but at 1024×700 the three order cards still squeeze their hints to one or two words a line and "Reset my order" wraps inside its pill. In the View captures the page heading "5 sessions need you" still peeks out half-covered above the menu's top edge.
