<!-- captures: 0c8ae4d62355 -->
# 77-menus: every row menu, the palette's action list and a submenu

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

The menus share one pattern: a small header naming the object, icon rows grouped by rules, and the destructive item (Remove…, Delete…, Forget…, Stop this session) last in red. In dark the menus are a lighter surface than the page and in light they are white with a shadow over the grey canvas, so they stand out in both. session-menu-greyed explains its disabled items in place ("It has no conversation to carry over yet"), and session-submenu opens "Snooze or mute" to the side with a focus ring on the first choice. palette-actions shows the actions list with Go to it highlighted and a cut row at the bottom hinting at more. The session menus are too narrow for their longest label: "Continue on another accou…" is cut beside its ⌘⇧H shortcut in session-menu, session-menu-greyed and session-submenu. The project menu in project-menu covers the rail rows it opens over, and account-menu's menu covers its own card's row.

## Weak

- session-menu-1280x800, session-menu-greyed-1280x800, session-submenu-1280x800 (both themes): "Continue on another account…" is cut to "Continue on another accou…".
- project-menu-1280x800: the menu sits over the rail rows it belongs to, cutting "Fake Desk app" to "ap".
