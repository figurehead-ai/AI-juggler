# Evidence manifest — AJ-E1507

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1507 (rules and MCP sync across tools, with a diff preview). Status: **IN-PROGRESS**. 2026-10-22, claude-opus.

## What changed

1. **Tool knowledge stays in the adapters.** Each tool's project rules files come from its `ToolSpec::rules_files`. A new `rules_import` field says how one of its rules files can read another instead of repeating it: Claude Code's `@{file}` line, or none for Codex. The app names no tool and no file.
2. **The comparison** (`juggler-handover` `sync`) says how each rules file stands beside the master:
   - the same text (line endings and blank lines at the end aside);
   - it reads the master through its tool's import line;
   - it's a link to the master;
   - it differs, by so many lines;
   - it's missing.

   A file that reads another can't be the master. With none picked, the master is the newest file with its own text. The line diff is `similar` 3.2 (Myers, with an 80 ms deadline).
3. **The commands** (`features/project_sync`): rules, diff, apply, servers, and the server copy. Only the two apply commands write, and only on the user's click.
   - **An apply writes one of the project's rules files and nothing else.** It refuses a link, a file that isn't text, and any file that changed since it was shown (both hashes are checked).
   - **What it replaces is kept for Undo:** the file from before goes to AI Juggler's data folder (`Undo::Restore`), and a file the apply created is removed by its undo.
   - **Undo works in two places:** right in the sheet, and later in Settings › Context packs & secrets. That list now names the project for a rules change.
4. **A missing file counts only for a tool that works in the project**, through a session or the history from before AI Juggler. A Codex account doesn't make every project without `AGENTS.md` differ.
5. **Servers** reuse the switch set-up (AJ-E1542) for any two accounts: the target tool's verdict, its own format, the record and the undo. Each server is offered once per account, from an account of the same tool first, so it's copied as it is where it can be. Secrets never move, and the note names what stays behind.
6. **The UI:**
   - The project's header says "Rules in sync" or "1 rules file differs". It opens one sheet, which every project menu also reaches ("Compare rules and servers…").
   - The sheet shows the diff first.
   - After any change, focus moves to the line that says what happened, so the keyboard keeps its place.

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 the rules files each tool reads, with when each changed and whether they differ | `project_sync` `nothing_is_written_until_the_user_applies` (standings and summary), `a_link_is_never_written`, `a_claude_md_that_reads_agents_md_is_in_sync_and_can_be_made_to`; `sync` unit tests (7) | PASS |
| Req.2 the user picks the master, sees a diff for the others, and only what's applied is written | `the_master_applied_writes_that_file_only_and_settings_undoes_it`, `a_file_changed_since_it_was_shown_is_never_overwritten`, `only_the_projects_rules_files_can_be_written`; vitest `project-sync` (8) | PASS |
| Req.3 MCP servers per account and tool, copied on request in the target's format; secret-looking values left out and named | `a_server_is_copied_across_tools_without_its_secret_and_undone`, `every_pair_of_tools_is_offered_what_it_lacks_the_same_tool_first` | PASS |
| Req.4 the project view says "Rules in sync" or "N files differ" | vitest "says a file differs, and opens the sheet", "reads the rules again when the sheet closes"; `a_missing_file_counts_only_for_a_tool_that_works_in_the_project`; gallery `82-project-sync/header-line-*` | PASS |
| Req.5 a guardrail proves nothing is written without an apply | `nothing_is_written_until_the_user_applies`: every file in the project and every account folder is snapshotted around the read-only calls. Proven able to fail: a planted write into the account folders fails it. | PASS |
| Budget: listing < 300 ms per project | `listing_stays_inside_its_budget` (two 2,000-line files, CPU time, best of five) | PASS |
| Budget: the diff < 100 ms for 2,000-line files | `sync` `two_thousand_line_files_diff_inside_the_budget` (CPU time, best of five) | PASS |
| Visual: the sheet, the diff and the project line | gallery `82-project-sync` (header line, the sheet with its diff, after applying and copying, the menu's way in), both themes, axe 0 critical/serious | PASS |

## Walked by hand

In the browser preview with the example data, I opened the project, clicked "1 rules file differs", applied CLAUDE.md to AGENTS.md, copied a server and pressed Escape. That walk found two bugs, both now fixed with tests:
- **Escape didn't close the sheet after Apply or Copy.** The button that was clicked disappears, and focus left the sheet with it. Focus now moves to the result line.
- **The header line kept its old words** when the sheet was opened from the project menu. The sheet now tells the page when it closes.

### On the real setup, in throwaway labs (`de40d664`, 2026-10-22)

A Dev build of this branch, walked by QA through the read-only Dev bridge. The full record is
walk batch 26 in `docs/qa/evidence/walks/2026-10-21-real-readonly.md` on
`claude/qa-real-walk-clean` (`16d84cc5`).

- **Setup**: the real setups were brought in where they are, with redacted labels. Two labs were
  made for the walk and are now in the Trash: one with a CLAUDE.md and an AGENTS.md that differ,
  and one with no rules files. They were added as projects in AI Juggler Dev's own data. No real
  session ran.
- **The reads, through the bridge** (all OK):
  - the header line, "1 rules file differs";
  - the sheet, from the line and from "Compare rules and servers…";
  - the diff;
  - "Reads AGENTS.md" for a CLAUDE.md of `@AGENTS.md`;
  - "A link to AGENTS.md" and, reversed, "A link to CLAUDE.md", with nothing to apply to either;
  - no line for the project without rules files;
  - Tab staying inside the sheet;
  - Escape closing it, with focus back on the line.
- **The servers against each real account's own files, read-only** (OK):
  - six Claude Code accounts show "None", and their state files hold no servers;
  - the Codex account shows its one enabled server, with a second, `enabled = false`, skipped on purpose;
  - each Claude account is offered the Codex server, translated.
- **The writes** ran as `tests/qa_sync_lab.rs` `sync_lab_writes` on a scratch copy of the lab, with
  fixture accounts and a stub tool (OK):
  - apply CLAUDE.md to AGENTS.md, so the files are equal and the summary says "Rules in sync";
  - Undo in the sheet, which puts the bytes back;
  - apply again, then Undo from Settings › Context packs & secrets, listed under the lab's name,
    which puts the bytes back;
  - no rows and no backups left.
- **Snapshots**: the real accounts had no path added or removed. Their configs changed only where
  the vendors write on their own, at the same size. AI Juggler Dev made no backups, so nothing was
  applied through it.

## Left

- **P22: the diff shows an unchanged last line as removed and added.**
  - `sync::normalized` trims each text's end, its final newline included, so a file's last line
    no longer equals the same line in the middle of the other file.
  - Example: AGENTS.md ending "Keep functions short." applied to a CLAUDE.md that has it in the
    middle reads "2 lines added, 3 removed"; the true diff is 1 and 2.
  - Fix: keep one final newline after the trim.
- **A gap not seen on the real setup:** Claude Code also loads a project's own `.mcp.json`
  (project scope), which `claude_inventory` doesn't read. No real project here has one.
- Focus after Apply is left to the gallery's `82-project-sync` "after applying and copying".
