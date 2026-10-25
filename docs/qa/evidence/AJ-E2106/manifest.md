# Evidence manifest — AJ-E2106

Task: `docs/tasks/E21-desktop-first.md` AJ-E2106 (menus and ⌘K with
icons and an order; the approved "desktop first" canvas, D-35 to D-37).
Status: **IN-PROGRESS**: the registry, every menu and ⌘K's action panel
are evidenced; ⌘K's apps wait for AJ-E2101's places, and the ↵ and Space
hints wait for AJ-E2102's keys. 2026-10-24, branch `e21-menus`.

Gallery: `docs/qa/evidence/_gallery/77-menus/` (the central run writes
the new states listed below).

## What exists now

- **The registry** carries an icon, a group and a rank for every action
  (`ui/src/lib/action-types.ts`, `action-specs.ts`,
  `action-specs-setup.ts`, read by `lib/actions.ts`).
  - Groups, in order: getting there, working on it, stopping (stopping
    and removing are red, last).
  - Rank: most used first within a group.
  - One submenu level: `SUBMENUS` (today one, "Snooze or mute").
  - The icons are one stroke set (`components/icons-actions.tsx` beside
    `icons.tsx`), mapped by name in `components/ActionIcon.tsx`. The
    same action has the same icon everywhere.
- **A session's menu**, as on the canvas:
  1. Go to it · Open its page · Show its project;
  2. Continue on another account… (⌘⇧H, D-05) · Rename… · Snooze for an
     hour · Snooze or mute › (For 15 minutes, Until the limit resets,
     Mute this session) · Copy the resume command · Pin to top;
  3. Stop this session (⌘.), red, last.

  Move to pool… is not there: pools are parked on their own branch
  (E1702), so no menu has it yet. The registry's `submenu` and `group`
  take it as one more entry when pools land.
- **Every menu is built from the registry:**
  - `ItemMenu.tsx`: icons, separators between groups, the submenu, and
    the greyed reason as the line's description. A right-click opens
    this in-app menu at the pointer (kept inside the window), as do the
    ⋯ button and Shift+F10 or the menu key (under the row);
  - `RowMenu.tsx`: the arrows move within one level, Home and End, → and
    ← for the submenu, Escape closes the submenu first;
  - `InspectorHeaderMenu.tsx`: shares `ON_ITS_PAGE` with ⌘K;
  - `native-menu.ts`, kept for the app's menu bar: the same order,
    groups and submenu, with the shortcut keys;
  - `MoreMenu.tsx`: each screen's icon.

  This covers the rail's rows, the project rows, Accounts' cards,
  History, This Mac, outside conversations and the page headers.
- **Why the right-click isn't the system's menu any more.** Tauri's menu
  items take a bitmap or one of AppKit's few named images, never a
  template image. A drawn icon would keep one colour on a highlighted
  line and in dark menus, and a system menu line can't be coloured. The
  approved design has icons, red stopping and greyed reasons, so every
  context menu is the in-app one (coordinator's ruling, 2026-10-24).
- **⌘K** (`GoToPalette.tsx`, extended, with `PaletteActions.tsx`):
  - with a session, project, account or tool selected, its actions come
    first: icon, label, greyed reason, and a shortcut hint (⌘., ⌘⇧H);
  - then the live sessions, as before;
  - then projects once something is typed;
  - "Open its page" is left out, as you're on it;
  - a greyed action says why and does nothing, and Enter on another runs
    it and closes the palette;
  - a rename sheet opened from it outlives it.

## Hide or grey

| Case | Rule | Where |
|---|---|---|
| Stop on a session another app started | left out (D-08) | conversation menu has no Stop |
| Stop, Go to it on an ended session | left out | session `applies: isLive` |
| Relocate… on a project whose folder is there | left out | project `applies: missing` |
| Snoozes while snoozed; Unsnooze while not | left out (one shows where the other was) | session |
| Show in Finder with no editor chosen | left out (Open in Finder is the line) | project |
| Continue on another account…, Copy the resume command with no conversation yet | greyed: "It has no conversation to carry over yet" / "to resume yet" | session, history |
| Until the limit resets with no known reset time | greyed: "No known reset time for this quota window" | session submenu |
| New session here, Open in…, Compare… with the folder gone | greyed: "Its folder isn't there any more" | project |
| Start a session on an account that can't now | greyed with the account's own reason | account |
| Continue in AI Juggler while it's open elsewhere | greyed: "It's open in another session now" | conversation |
| This Mac line AI Juggler didn't start | one greyed line: "AI Juggler only acts on sessions it started" | This Mac |

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 icon, group, rank on every action | vitest `item-menu.test.tsx` › "every action has an icon, a group and a rank": every spec of every type has a known group, a finite rank, a drawn icon; each icon name is drawn once; no two ranks collide in a group; the same label has the same icon across all types | PASS |
| Req.2 ≤ 3 groups, most used first, stopping last in red, one submenu level | vitest: groups in order for every sample item, ranks ascending, `destructive` exactly the stopping group; the submenu is one level; Stop is last after a separator with `text-danger`; → opens and ← closes the submenu; Playwright `menus-gallery` (session row, submenu) | PASS |
| Right-click opens the in-app menu | vitest: with a live backend present, a right-click opens the in-app menu at the pointer with its icons and red Stop, and no system menu is built; a click past the window's edge stays inside it; a contextmenu from the keyboard opens under the row. Playwright `menus-gallery`: right-click gives the same lines as ⋯ and Shift+F10, with icons, within 1 px of the pointer | PASS |
| Req.3 left out vs greyed with the reason | vitest: ended session (no Go to it, no Stop; greyed carry-over and resume), conversation (no Stop), moved project, the greyed line's `aria-describedby` is its reason; table above | PASS |
| Req.4 ⌘K: the item's actions with shortcuts, then sessions, apps, projects | vitest `palette-actions.test.tsx`: actions first with ⌘. and ⌘⇧H, then sessions, projects on typing; greyed does nothing; Enter runs and closes; nothing selected is as before. Apps: not yet (AJ-E2101 has no places to list) | PARTIAL |
| a11y: menu roles, focus order, icons hidden, contrast | vitest: role=menu, focus on the first line, arrows/Home/End, focus back on Escape; every icon `aria-hidden`. Contrast (computed from the tokens): greyed text `--color-faint` 5.44 (dark) and 5.56 (light) on the menu, 5.08 and 5.05 on surface-2, 4.68 and 4.97 on ⌘K's active row; red 6.67 and 6.47 | PASS |
| Performance: menu opens < 50 ms | vite dev server, 10 right-clicks timed from the event to the next painted frame with the menu in it: 16.1 ms the first time, 3.7–6.3 ms after | PASS |
| Visual: each menu, both themes | `menus-gallery.spec.ts` (session menu, submenu, greyed, ⌘K, project, project header, account, history), `home-gallery` (More), `project-manage-gallery`, `account-manage-gallery`; run on this Mac outside the gate and all passed; the Docker gate is run centrally | PENDING (central run) |

Checks on the branch: `npx vitest run` 114 files and 867 tests pass;
`npm run typecheck`, `plain_words_lint.py`, `terminology_lint.sh`,
`size_budget.py` and `just structure` (layering, IPC audit, size) are OK.

## Walked by hand

The vite dev server with the preview fixture, at 800×600, both themes:

- a right-click on a running session showed Go to it, Open its page,
  Show its project; Continue… (⌘⇧H) through Pin to top; Stop this
  session (⌘.) in red;
- hovering Snooze or mute opened its three lines beside it;
- an ended session's menu had no Go to it and no Stop;
- ⌘K on that session listed its actions first and Live sessions after;
  "rena" then Enter closed the palette and opened the rename sheet,
  closed again with Escape.

## Baselines the central run will change

- gallery `77-menus/session-menu-1280x800-dark.png`
- gallery `75-account-manage/account-menu-1280x800-dark.png`
- gallery `72-tool-view/header-menu-1280x800-dark.png`
- regression `account-menu-{dark,light}-comfortable-1280x800.png`
- regression `project-menu-{dark,light}-comfortable-1280x800.png`

New gallery states, not gated: `77-menus/session-submenu-*`,
`session-menu-greyed-*`, `palette-actions-*` (both themes).

## Not done

- ⌘K's apps (places): AJ-E2101 brings the app and profile each session
  runs in.
- The ↵ and Space hints beside Go to it and Open its page: those keys
  don't do that on a row yet (Enter opens its page), and AJ-E2102 wires
  them. A hint that isn't true isn't shown.
- The right-click in the built app (a real WebView) is checked by the
  maintainer on the replica; the preview and the specs show the in-app
  menu at the pointer.
