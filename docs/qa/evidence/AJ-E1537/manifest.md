# Evidence manifest — AJ-E1537

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1537 (context menus and ⋯ menus
from one action registry; the maintainer's request, 2026-10-21; built
with AJ-E1502). Status: **DONE-EVIDENCED**. 2026-10-21, reviewer
claude-opus.

Gallery: `docs/qa/evidence/_gallery/77-menus/`. It has a session row, a
project row, a project page's header, an account card and a history
entry, each in both themes.

## What exists now

- **The registry** (`ui/src/lib/actions.ts`). Each item type declares its
  actions once: session, project, account, tool, history entry and
  This Mac row. Each action has an id, a label, a group (main,
  attention, destructive), a shortcut from the shortcut registry, when
  it applies, why it's blocked, and what it does. `resolveActions`
  orders them with destructive ones last.
- **Capabilities, not copies.** An action does its work through the
  screen's `ActionHost`:
  - App: open, focus, continue on another account, stop, copy the resume
    command, new session here, the account and tool pages;
  - the rail: snooze and mute;
  - ManageWork: rename, pin, open in the editor, copy the path,
    relocate, forget;
  - `useAccountManage`: rename, default, remove;
  - History: replay, archive, delete.
  An action whose capability isn't there doesn't show. No screen keeps
  a menu of its own: the rail's, Accounts' and ManageWork's
  hand-written menus are gone.
- **Three triggers, one menu** (`components/ItemMenu.tsx`):
  - a right-click opens the system's own menu (`lib/native-menu.ts`,
    Tauri's menu popup) in the app, and the in-app menu in the preview;
  - each row's ⋯, and the page header's;
  - Shift+F10 or the menu key on a focused row.
  The in-app menu measures itself and stays on screen.
- **Where:**
  - rail session rows and project rows (headers and the project list);
  - Accounts cards;
  - History rows;
  - This Mac's lines;
  - the page header for sessions, projects, accounts and tools ("Open"
    is left out there, since you're on it).
- **New actions:**
  - **Copy resume command:** `sessions_resume_command` takes the
    session's id, and the tool's adapter builds the line from its own
    `resume_args` (`claude --resume <id>`, `codex resume <id>`), on the
    account's folder, from the session's folder, every word quoted. No
    tool's command is written in shared code.
  - **New session here:** New session opens on the project's folder.
  - **Copy path.**
  - **Copy update command,** a tool's install command. ADR-0003: the
    app prepares, the user runs it.
  - **Not started by AI Juggler** on This Mac: a disabled line saying
    so, with no ⋯ (D-08).
- **Disabled items say why.** In the in-app menu that's a line under the
  item and its tooltip; in the native menu, a disabled line under it.
  Their `run` does nothing.
- **Shift+F10** is on the `?` sheet ("Open a row's menu").

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 one registry, actions declared once, features contribute | `lib/actions.ts`; vitest `item-menu.test.tsx`: each type's menu pinned (a live session, an ended desktop one, a project and a moved one, an account, a tool, a history entry, This Mac's lines); an action whose capability is missing doesn't show | PASS |
| Req.2 three triggers, one menu; native on right-click | vitest: ⋯, right-click, Shift+F10 and the menu key list the same items in the registry's order; the native menu gets the same labels, groups, disabled items and reason lines (`@tauri-apps/api/menu` mocked). Playwright `menus-gallery`: on the same row, ⋯, right-click and Shift+F10 give the same list, and Escape gives focus back | PASS |
| Req.3 each type's items | vitest (above); gallery `77-menus` (session, project, project header, account, history) | PASS |
| Req.4 destructive last and apart, with confirm or undo; outside sessions read-only | vitest: destructive last in every type, after a separator; Stop goes through `holdStop` (5 s with Undo, AJ-E1503); Forget, Remove and Delete open their confirm sheets. A tool AI Juggler didn't start gets only the disabled "Not started by AI Juggler" line (`this-mac.test`); sessions started outside AI Juggler arrive with AJ-E1529 | PASS |
| Tests: keyboard, a11y (role=menu, focus return), disabled never runs | vitest: role=menu named "Actions for …", focus on the first enabled item, arrows, Escape returns focus; a disabled item's handler never runs. axe 0 critical/serious on every gallery menu. A disabled item's reason sits outside the item, so its name is its label (axe had caught label-content-name-mismatch) | PASS |
| Copy resume command | Rust `codex_adapter :: the_resume_line_runs_from_the_session_folder` and `claude_code_adapter :: the_resume_line_names_the_conversation` (shape, quoting, the session's folder, option-like ids refused) | PASS |

## Walked by hand

The vite dev server with the preview fixture, at 800×600:

- the rail's ⋯ on a running session: Open, Show its terminal, Continue
  on another account… (⌘⇧H), Rename…, Pin to top, Copy resume command,
  Show its project, then Snooze and Mute, then Stop (⌘.) in red;
- Show its project opened the project page, and its header ⋯ listed
  the project menu without Open, with Forget last;
- New session here opened New session on that project's folder;
- Shift+F10 on a focused session row opened its menu with focus on
  Open, and Escape put focus back on the row.

Right-click in the real app opens the native menu, which the preview
can't show. The maintainer checks it on the replica (0.1 acceptance §2).
