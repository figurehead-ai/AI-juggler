<!-- captures: 4aabf22558c7 -->
# 01-rail-sort: the rail's sort and grouping options (needs me, by project, quota risk), a search with no match, a snoozed row and the rail's View menu

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

empty-search-1440x900 handles a search with no match well: a focused field, "No sessions match" and a "Clear filters" button. view-1440x900 opens the new View menu with icons and short hints ("Inactive · in an app's list, not running"), a check mark on Ended today and "Only Account/Project · All ›"; it covers the page heading and the rail's Account tab while open, which is expected of a popover. The sort captures do not show their sort: group-by-project-1440x900 and needs-me-1440x900 are pixel-for-pixel the same as the default Home, and nothing on screen names the active sort or grouping. snoozed-1440x900 shows "snoozed 12m" as a small grey chip hanging over the boundary between two rows, where it reads as belonging to either. quota-risk-1440x900 was not re-rendered and still shows an older layout (Home, Accounts, History, Settings, More in the header, old filter chips and sort selects), so it doesn't match the rest of the family.

## Weak

- group-by-project-1440x900, needs-me-1440x900: identical to the default Home; the active sort or grouping is not visible anywhere.
- snoozed-1440x900: the "snoozed 12m" chip sits between two rows, not inside the row it belongs to.
- quota-risk-1440x900: not re-rendered; shows an older header and filter layout.
- all rail captures: "Stopped with an error · 7…" cuts its age.
