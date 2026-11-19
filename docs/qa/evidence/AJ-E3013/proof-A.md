# AJ-E3013 lab proof A: a plugin in a Claude Code config folder's `skills/` (2026-11-16)

Research §7.1 (`docs/research/desktop-pool-members.md`). It ran headless on AI Juggler Dev's sanctioned Lab App folder (`…/AI Juggler Dev/accounts/claude-code/claude-code-<id>`, a signed-in lab account), from a throwaway `~/lab` project. A Sonnet driver ran it with exact commands, and each verdict was checked by Opus. The server was the lab probe (`juggler-agent --mcp --probe`, debug only, c3d68ca12): it records environment variable names (never values), ids, the parent chain, the client and `_meta` keys. The plugin was named `ai-juggler-probe`, so it could not touch the real connector.

| Step | Verdict | Evidence |
|---|---|---|
| L1: loaded from `skills/` | PASS | `claude plugin list --json` listed `ai-juggler-probe@skills-dir`, enabled, with its `juggler-probe` server; no prompt |
| L2: its tool listed and called, no prompt | PASS | the init showed `plugin:ai-juggler-probe:juggler-probe` connected (source plugin); the tools held `mcp__plugin_ai-juggler-probe_juggler-probe__probe`; one call returned "recorded"; `permission_denials` was empty |
| L4: its hook reaches the session | PASS | `UserPromptSubmit` printed "AIJ-LAB-NOTE-hook-ran", exit 0, outcome success. It shows in `-p` stream output only with `--include-hook-events` |
| L5: Remove is exact | PASS | after the folder was removed, the plugin was gone from the list; `settings.json` sha256 identical before and after; `skills/` names unchanged |
| A9 (the environment): does a plugin's MCP server get the session's inbox? | **YES** | Run again with both variables removed from the launching environment (`env -u CLAUDE_CODE_MESSAGING_SOCKET -u CLAUDE_CODE_MESSAGING_TOKEN`, confirmed 0 before): the probe's environment names still held `CLAUDE_CODE_MESSAGING_SOCKET` and `CLAUDE_CODE_MESSAGING_TOKEN`, so Claude Code itself gives them to a plugin's server. The probe file was checked by Opus: 6 records, the `tools/call` recorded, no value longer than an id. |

**Also seen:**
- The server's environment holds `CLAUDE_PLUGIN_ROOT`, `CLAUDE_PLUGIN_DATA`, `CLAUDE_PROJECT_DIR`, `CLAUDE_CONFIG_DIR`, `CLAUDE_CODE_SESSION_ID` and `CLAUDE_CODE_HOST_SESSION_ID`. The parent chain reaches `claude` in one hop.
- The client is `claude-code` v2.1.322.
- The `tools/call` `_meta` keys are `claudecode/toolUseId` and `progressToken`.

A run with `env -i` could not sign in ("Not logged in"), because the lab account's sign-in needs the user's environment; it was not used as evidence.

**Clean-up:** the probe plugin folder was removed and `~/lab/aj46*` deleted; no probe or `claude -p` process was left. `claude plugin list` afterwards also showed `discord@synced` (the account's own synced plugin, not touched).

**What this settles:**
- Route A (the connector's own folder in `skills/`) loads with no prompt and is removed exactly.
- D-119's waking route can work: the connector's server holds its own session's inbox socket and token.
- The end-to-end proof (an attested desktop session, messages, a task, an idle session woken) is the aij-test proof (§7.2).
