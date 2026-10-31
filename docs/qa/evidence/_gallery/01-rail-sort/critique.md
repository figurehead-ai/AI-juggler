<!-- captures: ef795d0c8f68 -->
# 01-rail-sort: the rail's sort and grouping options (needs me, by project, quota risk), a search with no match, a snoozed row and the rail's View menu

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

empty-search-1440x900 handles a search with no match well: a focused field with a violet ring, "No sessions match" and a "Clear filters" button. view-1440x900 opens the View menu with icons and short hints ("Inactive · in an app's list, not running"), a check mark on Ended today and "Only Account/Project · All ›"; it covers the page heading and the rail's Account tab while open, which is expected of a popover. The lighter dark accent shows on the logo mark, the search ring, "Open its page" and the Work Max runway bar, and the top bar stays one row. The rail's error rows now read "Stopped with an error" without a cut age. The sort captures still do not show their sort: needs-me-1440x900 is pixel-for-pixel the default Home of 01-rail, group-by-project-1440x900 looks the same, and nothing on screen names the active sort or grouping. snoozed-1440x900 still shows "snoozed 12m" as a small grey chip on the boundary between the "Limit reached · 0% left" row and the "Asks you · 2 h" row, where it reads as belonging to either. quota-risk-1440x900 was not re-rendered and still shows an older layout (Home, Accounts, History, Settings, More in the header, old filter chips and sort selects), so it doesn't match the rest of the family.

## Weak

- group-by-project-1440x900, needs-me-1440x900: identical to the default Home; the active sort or grouping is not visible anywhere.
- snoozed-1440x900: the "snoozed 12m" chip sits between two rows, not inside the row it belongs to.
- quota-risk-1440x900: not re-rendered; shows an older header and filter layout.
