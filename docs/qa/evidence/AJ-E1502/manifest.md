# Evidence manifest — AJ-E1502

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1502 (manage sessions and projects). Status: **DONE-EVIDENCED**. 2026-10-21, reviewer claude-opus.

## What changed

1. **Sessions** can be renamed and pinned from the rail row's ⋯ menu (and right-click), and from the session view's Actions tab.
   - The name is a label shown over the session's own title (its branch, else its folder). Leaving it empty brings the title back. Search finds the label.
   - Pinned sessions lead their group in every sort, each with a pin mark (and ", pinned" for screen readers).
2. **Projects** get the same row menu: on a project's group header in the rail and on its row in the rail's project list. The project view's Actions tab ("This project") offers the same actions.
   - **Rename…** sets a label only; the folder is never renamed. Leaving it empty uses the folder's name.
   - **Pin to top**: a pinned project's group leads the rail.
   - **Open in …**: the editor chosen in Settings › General ("Open projects in", only editors this Mac has), else Finder. **Show in Finder** sits beside it when an editor is chosen.
   - **Relocate…** points a moved folder at its new place, from the menu, the Actions tab or the missing-folder banner (the hidden action from AJ-E1025, shown again now that it works).
   - **Forget…** hides the project and its sessions from the rail and New session until it is used again. Its sessions stay in History. It is refused while a session there is open, naming it.
3. **Storage** (schema v31): `instances.label`, `instances.pinned_at`, `projects.pinned_at`, `projects.forgotten_at`. A project's label is its existing `name`. All of it is in the database, so it survives restarts and backups.
4. **"Used again"** means a session started there (both launch paths clear the forget), or newer work there outside AI Juggler (its imported history is newer than the forget).
5. **Found on the hand walk and fixed.** The rail's `backdrop-blur` makes it the containing block for fixed elements, so a sheet opened from the rail was squeezed inside the rail. The row menu and these sheets now render on the page itself (a portal). The gallery's `fill()` had passed regardless.

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 session rename (a label over its title) | Rust `manage_projects :: a_session_label_shows_over_its_title_and_a_pin_marks_it` (set, blank back to the title, too long refused, unknown id). vitest `project-manage.test.tsx` "a label shows over the session's own title…" and "renames (blank goes back to the title) and pins". Gallery `rename-session-*`. | PASS |
| Req.1 session pin to the top | Same Rust test; vitest "pinned sessions lead in every sort…"; Playwright regression `rail-pins-*`; gallery `rail-pins-*`. | PASS |
| Req.2 project rename (folder never renamed) | Rust `a_project_rename_is_a_label_and_the_folder_keeps_its_name` (the folder is where it was; no folder of the new name; blank gives the folder's name). vitest "rename keeps the folder's name as the fallback…". | PASS |
| Req.2 pin | Same Rust test; vitest "a pinned project's group leads the rail, and its row leads the history list". | PASS |
| Req.2 open in the editor, else Finder; reveal | Rust `a_project_opens_in_the_chosen_editor_else_finder` (no choice: Finder; only installed, known editors offered and accepted; Zed; reveal; back to Finder). vitest "offers rename, pin, open…" and "with an editor chosen…". Gallery `project-actions-*`, `settings-editor-*`. | PASS |
| Req.2 forget until used again | Rust `a_forgotten_project_is_hidden_until_it_is_used_again`: refused with an open session (named); forgotten drops its pin and keeps its session; a launch there brings it back; newer history brings it back; the undo. vitest "…forget asks first and shows a refusal in words". Gallery `forget-project-*`. | PASS |
| Req.2 relocate, with a missing folder | Rust `relocate_repoints_a_moved_folder_and_refuses_a_missing_or_taken_one` (missing → opening refused in words; a folder that isn't there or that another project has is refused; relocated: not missing, the new root, its name kept). vitest "a missing folder can't be opened, and Relocate points it…" and `inspector-aggregate` (Relocate… on the banner). Gallery `relocate-*`. | PASS |
| Req.3 row menu and project view, AJ-E1025's hidden actions back | The rail row menus (vitest above); the project view's "This project" block and the banner's Relocate… (gallery `project-actions-*`, `relocate-*`); the session view's Rename… and Pin to top. | PASS |
| Req.4 survive restarts and backups | Rust `labels_and_pins_survive_a_restart_and_a_backup` (reopened on the same data; a backup restored on another Mac). | PASS |
| Budgets | Each action timed < 100 ms in the Rust tests. The rail's sort keeps its 200-row budget (`sorting.test.tsx`, unchanged, passing with the pinned partition). | PASS |

Gallery: `docs/qa/evidence/_gallery/76-project-manage/` (16 captures, axe 0 critical/serious on each state).

## Walked by hand

Browser preview against this UI (1024×768, dark):
- pinned a session from its row menu (it moved to the top with a pin mark);
- renamed it with the keyboard only (menu → Enter on Rename… → type → Enter): this is where the clipped sheet showed up, now fixed;
- pinned a project from its group header (its group moved to the top);
- forgot it (its group and sessions left the rail);
- chose Zed in Settings: the project's Actions tab then offered "Open in Zed" and "Show in Finder".

The Dev build's window can't be clicked from an agent session on this Mac (no Accessibility grant). The commands behind it are the ones the Rust tests call.

## Not covered here

- `open -a <editor>` and the Finder reveal themselves (Tauri's opener) weren't run against real apps: that would open windows on the maintainer's Mac. The choice of target is tested.

## On release/0.1 (2026-10-21)

The branch was merged into release/0.1 (`cec81e46`, after main's #108)
and finished under its rules:

- **IPC (ENGINEERING-GUIDE T1).** The nine new commands are async, with
  their work in `ipc::blocking`.
- **No paths from the webview (SECURITY C1).** `project_relocate` takes
  only the project's id. The backend opens the system's own folder
  dialog, so the path never passes through the UI, and returns `false`
  when the user cancels. The UI test now pins that it is called with the
  id alone.
- **One IPC module per feature (U3).** Its calls moved to
  `ui/src/lib/ipc/manage.ts`, and Settings' editor card to
  `components/EditorCard.tsx`, so bridge.ts and Settings.tsx stay within
  their size caps.
- **One menu (AJ-E1537).** The row menus for sessions and projects now
  come from the action registry. The rail's rows, the project page's
  header ⋯, a right-click and Shift+F10 all list the same items, so the
  inspector's Actions tabs no longer repeat Rename and Pin, or the
  "This project" block. The missing-folder banner keeps its
  Relocate….
- **Schema 31.** `SCHEMA_RELEASES` names it "0.1.x", with its row in
  docs/RELEASING.md § Database changes. The downgrade note goes in 0.1's
  release notes, and the version is filled in when 0.1 is cut.

The gates ran on the release head (see AJ-E1537's manifest: the same
run).
