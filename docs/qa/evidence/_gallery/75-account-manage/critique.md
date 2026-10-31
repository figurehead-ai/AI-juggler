<!-- captures: 2f769a94e858 -->
# 75-account-manage: renaming and removing accounts on the Accounts page

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

The Accounts page (default, account-menu, renamed-accounts) now separates its account cards from the page in both themes, with dark cards a step lighter than the canvas and white cards on grey in light; the long "No current reading…" sentence is brighter than the labels. The row menu in account-menu is a raised surface with icons and a red Remove at the bottom, set apart by a rule, though it covers the card it was opened from. The dialogs read well: remove-copy offers two radio cards with the chosen one tinted violet, remove-in-place and remove-refused state plainly what happens, and rename-duplicate puts the clash in red text above a solid violet Save; Remove stays a quiet red outline, which keeps the destructive step from shouting. renamed-rail and renamed-usage carry the new name ("Side projects", "Max (personal)") through the rail, the legend and the chart without layout changes. Two things are off: the monogram for "Max (personal)" is "M(" in renamed-accounts, and the default and account-menu captures open scrolled with the first card's top cut under the header.

## Weak

- renamed-accounts-1280x800 (both themes): the profile monogram for "Max (personal)" is "M(", a bracket taken as a letter.
- default-1280x800, account-menu-1280x800, renamed-accounts-1280x800: the page is captured scrolled, so the top card's header is cut under the app header.
- account-menu-1280x800: the menu covers its own card's row, so it is not obvious which account it belongs to beyond its small "personal-max" header.
