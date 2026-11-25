<!-- captures: 78bfc26ef4f2 -->
# 01-rail-sort: the rail's sort and grouping options (needs me, by project, quota risk), a search with no match, a snoozed row and the rail's View menu

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

empty-search-1440x900 handles a search with no match well: a focused field with a violet ring, "No sessions match" and a "Clear filters" button. view-1440x900 opens the View menu with icons and short hints ("Inactive · in an app's list, not running"), a check mark on Ended today and "Only Account/Project · All ›"; it covers the page heading and the rail's Account tab while open, which is expected of a popover. The lighter dark accent shows on the logo mark, the search ring, "Open its page" and the Work Max runway bar, and the top bar stays one row. The rail's error rows now read "Stopped with an error" without a cut age. The sort captures still do not show their sort: needs-me-1440x900 is pixel-for-pixel the default Home of 01-rail, group-by-project-1440x900 looks the same, and nothing on screen names the active sort or grouping. snoozed-1440x900 still shows "snoozed 12m" as a small grey chip on the boundary between the "Limit reached · 0% left" row and the "Asks you · 2 h" row, where it reads as belonging to either. quota-risk-1440x900 was not re-rendered and still shows an older layout (Home, Accounts, History, Settings, More in the header, old filter chips and sort selects), so it doesn't match the rest of the family.

## Weak

- group-by-project-1440x900, needs-me-1440x900: identical to the default Home; the active sort or grouping is not visible anywhere.
- snoozed-1440x900: the "snoozed 12m" chip sits between two rows, not inside the row it belongs to.
- quota-risk-1440x900: not re-rendered; shows an older header and filter layout.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5)

Opened at full size: view, snoozed (before and after the fix) and empty-search, 1440x900 dark (the family's only size and theme). The View menu (Show: Inactive, Quiet ones, Ended today ✓, Background runs, Other Macs, Archived; Only: App, Account, Project › All) is one overlay surface with an icon and a muted hint on every line, as the calm kit's menus.

- **Since 0.3.2:** the sorts left with the old Filters (AJ-E2104: the rail's View groups and shows; Home's Runway reads the room left), so group-by-project and needs-me are the Project lens chosen at the top, which now says what's active (the 0.3.2 weak point, "the active grouping is not visible", is answered by the lens). empty-search says "No open or recent session matches “does-not-exist”", offers "Search all history for …" and Clear filters.
- **Fixed** (1b87b41cc): the "snoozed 12m" badge sat over the boundary of two rows, on the title of "Draft the migration plan". Now the row says it in its own line: "Asks you · snoozed 12m" (the age yields first). Re-captured (snoozed-1440x900, local) and checked by eye; sorting.test checks the words sit in the row.
- **Removed:** quota-risk-1440x900, the old quota sort's capture, which no test had written since AJ-E2104 (F62).
- The desktop row's cut state word (F55) shows here too.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | Narrow the rail by App, Account or Project, show what's hidden, find a session; each says its state. |
| 2 | Prominence | 4 | Lenses on top, View behind the filter button, search first. |
| 3 | Default | 4 | Ended today on by default with a ✓; Reset beside View. |
| 4 | Mode | 4 | Calm; the menu on the overlay tone with a soft shadow. |
| 5 | Mock-up | 4 | Matches calm-ui's rail and the row menus (77-menus). |
| 6 | Areas | 4 | After the fix: the snooze is in its row's line; F55's cut word now ends on "…". |
| 7 | Colour | 4 | Accent only for the focused search's line and the chosen check. |
| 8 | Type | 4 | Menu lines 13 px, hints 12 px muted. |
| 9 | Actions | 5 | No primary; Clear filters is a quiet button. |
| 10 | Serious | n/a | Nothing destructive. |
| 11 | Doors | 4 | Search all history opens History with the words. |
| 12 | Words | 4 | "Inactive · in an app's list, not running": each option says what it shows. |
| 13 | States | 4 | No match said once with two ways on. |
| 14 | Consistency | 4 | The View menu is the row menus' pattern (icons, hints). |
| 15 | Accessibility | 4 | Not re-walked by hand this round; keyboard names and focus as the a11y gate checks them. |
| 16 | Hand walk | 4 | The PL1 walk (AJ-E2323) drove Home and the rail in Dev; not re-walked here. |
| 17 | System prompts | 5 | None. |
| 18 | Usability | 4 | One place to narrow the list; nothing on prime space that's rare. |

Lowest: 4 (line 6 was 3 before the fix).

**Re-captured after F55 (2026-11-24, Opus):** all five re-rendered and opened (needs-me, group-by-project, snoozed, view, empty-search). The desktop row's state word reads "Stopped with an …" (F55, it was cut mid-letter); the chosen lens is F53's solid fill. Nothing else moved. Lowest stays 4.

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: Needs you's cards are tone fills with their level's mark (a stopped one in the broken tone), the most urgent first (F53, F66, F80); the lens switch's chosen segment is a solid fill; rail rows carry a model mark where the model changed (F77). Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.

## 0.5.2 (2026-11-25, AJ-E1557, AJ-E1556)

Re-captured needs-me, the View open, group by project, snoozed and empty search. What changed: no dot before an idle or ended row, the gutter kept so titles hold their place (AJ-E1557); empty search changes only behind the rail. Home behind it also shows what HEAD already drew and this capture had not caught up with: Needs you now's two stopped sessions in one tinted block, the limit card apart below it (0.5.1's a11y batch). Each re-captured state was diffed against its last capture (pixel counts per region: the rail, the top bar, the content) and opened by eye, old and new side by side, both themes. Nothing broken. Scores unchanged; the rail is calmer: the eye goes to the four marked states.
