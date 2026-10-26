# Evidence manifest — AJ-E1905

Task: `docs/tasks/E19-agent-tasks.md` AJ-E1905, a Claude Code plugin (P2).
Status: **IN-PROGRESS**: built and tested in a fixture home; not published until D-43 point 6 is ruled. 2026-10-25, claude-opus.

Design: `plugins/claude-code/` (a marketplace listing one plugin, `ai-juggler`: its manifest, one MCP server run from the plugin's own folder by `bin/juggler-mcp`, and the `pool-tasks` skill), `crates/juggler-agentbridge/src/plugin.rs` (`juggler-agent --mcp --plugin`). Test output: [plugin-tests.txt](docs/qa/evidence/AJ-E1905/plugin-tests.txt).

## What exists now

1. **The bundle**: `plugins/claude-code/.claude-plugin/marketplace.json` and `ai-juggler/` with `.claude-plugin/plugin.json` (version 0.2.0, the app's), `.mcp.json` (`${CLAUDE_PLUGIN_ROOT}/bin/juggler-mcp`), and `skills/pool-tasks/SKILL.md` (when to list, claim, update, give back; what only the user does).
2. **The server**: `bin/juggler-mcp` finds AI Juggler's own `juggler-agent` (the launch's `JUGGLER_AGENT`, else the installed editions' bundles) and runs it as `--mcp --plugin`. In a session AI Juggler started (its key in the environment) it lists no tools, since the launch already added them; anywhere else every call says the session wasn't started by AI Juggler with its agent tools, or that AI Juggler isn't running. With no AI Juggler on the Mac it says so and exits. It never reads, writes or carries a key.

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 a plugin bundling the bridge's MCP config and a task skill | `plugin.rs::the_plugins_manifests_are_what_claude_code_reads_and_hold_no_secret` | PASS |
| Req.1 published with the app's releases | the release workflow doesn't package it yet | UNPROVEN (waits on D-43 point 6) |
| Req.2 works only while AI Juggler is running, and says so otherwise | `plugin.rs::installed_in_a_fixture_home_it_runs_the_bridge_and_says_why_not` (the real `juggler-agent`: the tools and why not outside AI Juggler's launches; nothing listed twice inside them; no AI Juggler: says so); `plugin::tests` (2) | PASS |
| Goal: "even outside AI Juggler's launches" | ADR-0020 gives a session its identity only with a launch; D-30 says sessions started elsewhere can't call the tools | UNPROVEN (a ruling: D-43 point 6) |
| Live: the plugin in a real Claude Code session outside AI Juggler | 2026-10-26, `claude -p --plugin-dir plugins/claude-code/ai-juggler` (session-only, nothing installed) on the lab profile, the server pointed at this branch's `juggler-agent` ([AJ-E1902/lab-2026-10-26.md](docs/qa/evidence/AJ-E1902/lab-2026-10-26.md), D-43 point 6): connected; all 14 tools listed and the skill `ai-juggler:pool-tasks`; `tasks_list` answered "This session wasn't started by AI Juggler with its agent tools, so it has no key. Continue it in AI Juggler to use them."; nothing of the plugin's written to the profile | PASS (as built) |
| Gate: a plugin install test in a fixture home | `installed_in_a_fixture_home_…` (copied as Claude Code's plugin cache keeps it, run as its `.mcp.json` says) | PASS |

The labs' recommendation for D-43 point 6 (in that file): don't publish. AI Juggler's own Claude launches load a skill-only copy with `--plugin-dir`, which installs nothing. If a published plugin is still wanted, its server should list no tools when there is no key.

Last updated: 2026-10-26.
