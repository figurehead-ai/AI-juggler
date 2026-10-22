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

## Left

- **A walk of the Dev build, in a throwaway lab project.** Compare, apply and undo the lab's own rules files. List the servers read-only; nothing is copied to the real accounts.
