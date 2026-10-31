<!-- captures: 820aa5a8f5cd -->
# 76-project-manage: project and session menus, forget, relocate, rename and pins

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

project-actions and project-menu show the project menu with icons, grouped by rules, and a red Forget… last; forget-project's dialog is short and keeps Forget a quiet red outline. relocate puts a red "Project directory is missing" banner with Relocate… above the overview cards, which stand off the canvas in both themes, but the Activity chart's date labels are still cut by the card's bottom edge. session-menu now shows its subject: the feat/read-model row's menu open in the rail (Open its page, Show its project, then Continue on another account… ⌘⇧H, Rename…, Move to pool…, Snooze, Copy the resume command, Pin to top), wide enough that "Continue on another account…" is whole; it runs a few pixels past the rail's right edge onto the page. project-menu still captures the rail scrolled so far that the App / Project / Account switch is half cut under the top edge. rename-session is now re-rendered: the label reads "Name" in sentence case instead of small caps and Save is solid violet in dark instead of pale. After the design fixes the dark accent is lighter on the Tokens this week sparkline, the selected radio cards in settings-editor and the rail's New session outline, and the rail's feat/read-model row reads "Stopped with an error" whole in project-actions, project-menu and session-menu. settings-editor's theme, density and order choices are radio cards with the chosen one tinted, and rail-pins shows pinned projects cleanly.

## Weak

- relocate-1280x800 (both themes): the Activity chart's date labels are cut by the card's bottom edge.
- project-menu-1280x800 (both themes): the rail's App / Project / Account switch is half hidden under the top edge.
- session-menu-1280x800 (both themes): the row menu is wider than the rail and overhangs its right edge onto the page by a few pixels.
