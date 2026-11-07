# Evidence manifest — AJ-E2106

Task: `docs/tasks/E21-desktop-first.md` AJ-E2106 (menus and ⌘K with
icons and an order; the approved "desktop first" canvas, D-35 to D-37).
Re-audited against the code, its tests and the gallery on
`release/0.4.1`, 2026-11-07. Built in E21 (ac0e41dc0, 0.1.3); ⌘K's apps
came with AJ-E2901's palette (c651cb653).

Status: every requirement evidenced. ⌘K's apps, open before (they waited
for AJ-E2101's places), are listed now. One note: the committed
`_gallery/77-menus/` captures date from 6a06e7483 (2026-10-31, before
a6756c0e1 renamed "Continue on another account…" to "Continue in another
app…" the same evening), so they show the old label; the vitest names
below assert the current registry. Re-capture 77-menus at the next
gallery run.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 every action in the registry carries an icon, a group (getting there, working on it, stopping) and a rank | `cd ui && pnpm -s exec vitest run src/components/item-menu.test.tsx`: "in the registry, for every type", "each icon name is drawn, and drawn once", "no two actions of a type share a rank in a group", "the same action looks the same everywhere: one label, one icon" (`ui/src/lib/action-types.ts`, `action-specs.ts`, `action-specs-setup.ts`; icons `components/icons-actions.tsx` via `ActionIcon.tsx`) | every spec of every type: a known group, a finite rank, a drawn icon | pass | PASS | ac0e41dc0 | 2026-11-07 | Claude (self) |
| R2 menus show at most three groups, most used first, stopping last in red; one level of submenus | `item-menu.test.tsx`: "a menu has at most three groups, in order, stopping last and in red", "submenus are one level, and stay within their group", "destructive actions come last, in their own group, for every type", "Stop is last, after a separator, in red, with its shortcut", "a submenu opens with →, closes with ←, and its greyed line says why"; Playwright `tests/visual/menus-gallery.spec.ts` "a session row: ⋯, right-click and Shift+F10 open the same menu", "a session's submenu, from the keyboard"; gallery `_gallery/77-menus/session-menu-greyed-1280x800-dark.png` | ≤ 3 groups in GROUP_ORDER; ranks ascending; `destructive` ⇔ stopping; one submenu ("Snooze or mute": For 15 minutes, Until the limit resets, Mute this session) | pass; the capture (opened) shows the three groups with separators and "Stop this session ⌘." last in red (its carry-over line has the pre-rename label, see above) | PASS | ac0e41dc0 | 2026-11-07 | Claude (self) |
| R3 never possible here is left out; not possible right now is greyed with the reason | `item-menu.test.tsx`: "hide or grey: an ended session with no conversation", "D-08: a session another app started has no Stop at all", "a project, and a moved one", "an account, a tool, a history entry and This Mac's rows", "an action whose capability isn't there doesn't show", "a disabled action never runs"; Playwright `menus-gallery.spec.ts` "a session starting: greyed with the reason" | ended: no Go to it, no Stop; carry-over and resume greyed "It has no conversation to carry over yet" / "to resume yet"; moved project: Relocate…, New session here greyed; the greyed line's `aria-describedby` is its reason | pass | PASS | ac0e41dc0 | 2026-11-07 | Claude (self) |
| R4 ⌘K lists the selected item's actions with their shortcuts, then sessions, apps and projects | `cd ui && pnpm -s exec vitest run src/components/palette-actions.test.tsx src/components/palette-evidence.test.tsx`: "lists its actions first, with their shortcuts, then the sessions", "a greyed action says why and does nothing; Enter on another runs it and closes", "typing narrows the actions and finds projects", "an ended session has no Go to it and no Stop; nothing selected, no actions"; "an open app shows its own icon and launch badge, open ones first; ↩ opens its place, ⌘↩ brings it forward" (the Apps group); Playwright `menus-gallery.spec.ts` "⌘K with a session selected"; gallery `_gallery/77-menus/palette-actions-1280x800-dark.png` | the selection's actions first with ⌘. and ⌘⇧H, then the live sessions; typed: Apps (`goto-group-apps`) and Projects | pass; the capture (opened) shows "Actions for feat/read-model" with "Go to it ⌘↩", "Continue on another account… ⌘⇧H", "Stop this session ⌘." in red. Typed, the groups rank by match quality (`palette/rows.test.ts`, 0.4 design review 16), so Apps and Projects follow the actions in best-match order, not a fixed one | PASS | c651cb653 | 2026-11-07 | Claude (self) |
| Gate: a test that every action has an icon and a group; a11y menu roles | `cd ui && pnpm -s exec vitest run src/components/item-menu.test.tsx src/components/palette-actions.test.tsx src/components/palette-evidence.test.tsx` (run 2026-11-07); a11y: `item-menu.test.tsx` "is a role=menu that takes focus and gives it back on Escape", "every line has its icon, hidden from assistive tech"; Playwright `menus-gallery.spec.ts` (every menu, both themes) — runs in CI / the gallery gate | 3 files pass | 3 files, 45 tests pass; Playwright not run here (CI / the gallery gate) | PASS (vitest); CI for the gallery | ac0e41dc0 | 2026-11-07 | Claude (self) |

## What exists now

- **The registry** carries an icon, a group and a rank for every action
  (`ui/src/lib/action-types.ts`, `action-specs.ts`,
  `action-specs-setup.ts`, read by `lib/actions.ts`).
  - Groups, in order: getting there, working on it, stopping (stopping
    and removing are red, last).
  - Rank: most used first within a group.
  - One submenu level: `SUBMENUS` (today one, "Snooze or mute").
  - The icons are one stroke set (`components/icons-actions.tsx`),
    mapped by name in `components/ActionIcon.tsx`. The same action has
    the same icon everywhere.
- **A session's menu**: Go to it (⌘↩) · Open its page (↵) · Show its
  project; Continue in another app… (⌘⇧H) · Rename… · Move to pool… ·
  Snooze for an hour · Snooze or mute › · Copy the resume command · Pin to
  top; Archive (greyed while it runs, with why) · Stop this session (⌘.),
  red, last (`item-menu.test.tsx` "a live session: getting there, working
  on it, stopping").
- **Every menu is built from the registry:** `ItemMenu.tsx` (icons,
  separators, the submenu, the greyed reason as the line's description;
  right-click, ⋯, Shift+F10 or the menu key), `RowMenu.tsx` (arrows,
  Home/End, →/←, Escape), `InspectorHeaderMenu.tsx`, `native-menu.ts`
  (the app's menu bar: same order, groups, submenu and keys),
  `MoreMenu.tsx`.
- **Why the right-click isn't the system's menu.** Tauri's menu items take
  a bitmap or a few named AppKit images, never a template image, and a
  system menu line can't be coloured; the approved design has icons, red
  stopping and greyed reasons, so every context menu is the in-app one
  (coordinator's ruling, 2026-10-24).
- **⌘K** (`GoToPalette.tsx`, `PaletteActions.tsx`, `palette/rows.ts`):
  with a session, project, account or tool selected, its actions come
  first (icon, label, greyed reason, shortcut); then the live sessions;
  typed, Sessions · Apps · Projects · Accounts · Settings, ranked by match;
  "Open its page" left out on its own page.

## Hide or grey

| Case | Rule | Where |
|---|---|---|
| Stop on a session another app started | left out (D-08) | conversation menu has no Stop |
| Stop, Go to it on an ended session | left out | session `applies: isLive` |
| Relocate… on a project whose folder is there | left out | project `applies: missing` |
| Snoozes while snoozed; Unsnooze while not | left out (one shows where the other was) | session |
| Continue in another app…, Copy the resume command with no conversation yet | greyed: "It has no conversation to carry over yet" / "to resume yet" | session, history |
| Until the limit resets with no known reset time | greyed: "No known reset time for this quota window" | session submenu |
| New session here with the folder gone | greyed: "Its folder isn't there any more" | project |
| Continue in AI Juggler while it's open elsewhere | greyed: "It's open in another session now" | conversation |
| This Mac line AI Juggler didn't start | one greyed line: "AI Juggler only acts on sessions it started" | This Mac |

## Performance (from the first audit, 2026-10-24)

Menu opens < 50 ms: vite dev server, 10 right-clicks timed from the event
to the next painted frame with the menu in it: 16.1 ms the first time,
3.7–6.3 ms after. Not re-measured 2026-11-07.

## Binary artifacts

None added; cited: `_gallery/77-menus/` (session-menu, session-submenu,
session-menu-greyed, palette-actions, project-menu, project-header-menu,
account-menu, history-menu; both themes). The greyed and palette captures
were opened 2026-11-07; they predate the "Continue in another app…" rename.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (repo paths only)
- [x] Transcript excerpts scrubbed of personal content (none: fixtures only)
