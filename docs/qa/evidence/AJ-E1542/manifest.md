# Evidence manifest — AJ-E1542

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1542 (the capability preflight). Status: **DONE-EVIDENCED**. 2026-10-22, reviewer claude-opus.

## What changed

1. **Detection reads files and never starts a tool or a server** (`juggler-adapter` `inventory.rs`, per tool in `claude_inventory.rs` and `codex_inventory.rs`). It covers MCP servers, skills, commands and prompts, subagents, user rules and plugins.
   - **Claude:** user MCP servers, plus the project's private ones kept in the account's state file.
   - **Codex:** `config.toml`'s servers, plus skills in the home and in the shared `~/.agents/skills`.
   - **Project folder:** a project's own configuration (`.mcp.json`, `.codex/config.toml`, `CLAUDE.md`, `AGENTS.md`) lives in the folder both sides share, so it is never missing on the target and needs no set-up.
2. **The switch screen lists what the target lacks**, item by item (`SwitchSetupList`). Each says what will happen: "Set up as it is", "Set up in this tool's form", "Needs you: …", or "Can't be set up: …". Only exact, safe set-ups start ticked.
3. **Secrets are never copied.** A secret value in an env var or header becomes a named variable the target must provide (`${NAME}`, `env_vars`, `bearer_token_env_var`), and the screen names it. A sign-in never moves: the screen points at the tool's own login.
4. **Writes go through the tool where it has a command**, else as new files.
   - **Claude:** `claude mcp add-json`.
   - **Codex:** one appended `[mcp_servers.<name>]` table, checked to still parse. The file from before the first set-up is kept until the last undo, which removes the copy.
   - **Skills, commands, subagents and rules:** copied as new files, never over existing ones.
   - **Records and undo:** every write is recorded (`switch-setups.json` in AI Juggler's data) and can be undone from the switch's result ("Removed the skill release-notes"). The record is kept per set-up rather than in the switch's own manifest, because set-ups run before the switch, while its edge is still being made.
5. **ADR-0018** has an amendment for these approved writes.
6. **What can't be set up is listed with the reason:** a claude.ai-synced skill, a plugin across tools, an SSE server on Codex.

## Proof

| Requirement or gate | Proof | Result |
|---|---|---|
| Req.1 detection by parsing, both sides | `claude_inventory` `a_claude_account_lists_what_it_has_and_keeps_no_secret`; `codex_inventory` `a_codex_home_lists_what_it_has` | PASS |
| Req.2 the list, ticks only when exact and safe | Rust `switch_setup` `the_other_claude_account_is_offered_what_it_lacks_and_gets_only_what_is_picked`; vitest `switch-setup.test.tsx` "lists each item with what will happen, safe ones ticked" | PASS |
| Req.3 no secret copied | `codex_inventory` `a_claude_server_with_a_secret_arrives_as_a_forwarded_variable`; the Claude inventory test (a secret never reaches an item) | PASS |
| Req.4 recorded, through the tool's command, undoable | Rust `switch_setup` (both tests: the Codex config is byte-identical after the undo, and its backup is gone); vitest "can undo each set-up afterwards" | PASS |
| Req.6 "can't be set up there", with the reason | `codex_inventory` `an_sse_server_cant_go_to_codex`; the synced-skill row in both tests | PASS |
| Gate: no write without approval | The check writes nothing (`switch_setup`: the target has no `skills/` after the check). Only what's picked is written (no `commands/`). Vitest: nothing is set up until the switch runs. | PASS |
| Gate: translation fixtures per pair | `a_claude_command_becomes_a_skill`, `a_codex_prompt_becomes_a_claude_command`, the Codex MCP translation test | PASS |
| Gate: the lab with one MCP server and one skill on A only | Live labs 7 and 8 (`docs/qa/evidence/AJ-E1539/manifest.md`). Before each switch, both were offered and set up. B and then Codex used them from a fresh session ("KESTREL PEREGRINE"), which needs a working tool call and the skill. A wasn't offered what it had. Every set-up was undone, and A's, B's and Codex's configs matched copies taken before. | PASS |

## Not covered here

- A Codex MCP server set up on Codex asks for approval when first used, as Codex does for every server. The lab used Codex's automatic review (`--approve-for-me`) for its check.
