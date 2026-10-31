<!-- captures: 83be48c56e7f -->
# 77-menus: every row menu, the palette's action list and a submenu

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

The menus share one pattern: a small header naming the object, icon rows grouped by rules, and the destructive item (Remove…, Delete…, Forget…, Stop this session) last in red. In dark the menus are a lighter surface than the page and in light they are white with a shadow over the grey canvas, so they stand out in both. The session menus are now wide enough for their longest label: "Continue on another account…" is whole beside its ⌘⇧H shortcut in session-menu, session-menu-greyed and session-submenu. session-menu-greyed explains its disabled items in place ("It has no conversation to carry over yet"); its taller menu now opens from the top of the window and covers the rail's search, App/Project/Account switch and the start of the Needs you list, which is acceptable for a floating menu. session-submenu opens "Snooze or mute" to the side with a focus ring on "For 15 minutes". palette-actions now opens with the focus on its search field ("Act on feat/read-model, or search…", a violet ring), Go to it highlighted below, and a cut row at the bottom hinting at more; the ring is a square-cornered box inside the rounded palette, a little raw next to the app's rounded controls. In dark the lighter accent shows on the logo mark, New session and the palette's ring and Go to it. The project menu in project-menu still covers the rail rows it opens over, and account-menu's menu covers its own card's row.

## Weak

- project-menu-1280x800 (both themes): the menu sits over the rail rows it belongs to, cutting "Fake Desk app" to "app" and "Stopped with an error" to "Stopped with an e".
- palette-actions-1280x800 (both themes): the search field's focus ring is a square-cornered box, unlike the rounded rings elsewhere.
