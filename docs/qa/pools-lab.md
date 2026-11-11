# Pools labs (AJ-E3002)

> **Superseded for accounts (2026-11-11, ADR-0032's evidence):** a lab
> starts a tool only on the sanctioned test profile (`aij-test`), AI
> Juggler Dev's own Lab App folder, or a folder under a throwaway lab home,
> never on one of the maintainer's own tool folders (`~/.claude`,
> `~/.codex`, `ClaudeCodeProfiles/*`). Labs run here from 10-25 to 11-07
> started 62 Claude CLI sessions on two real profiles' folders, which then
> showed as "CLI" Apps on the maintainer's screens. AI Juggler Dev now
> refuses such a start (`lab_scope::refuses_real_folder`), and so does the
> switch lab.

Which of D-62's talk rules AI Juggler can actually enforce, from live runs.
A "no" is shown in the app as not enforced, never drawn as if it were.

## 1. Held and refused messages between sessions AI Juggler started — YES

Enforced by the agent bridge for each receiving session and each pool.

- Live, 2026-11-07, P2 on AI Juggler Dev with the Lab App (ADR-0027 7b):
  four Claude Code sessions in one pool, member → member "Ask first".
- w1's message to w2 was held. Claude said the message "stays queued until you deliver it
  in AI Juggler". Allow once delivered it to w2's inbox. Deny left w3's copy
  "Not sent", and it never reached w3. Record: `docs/qa/0.4-walks/P2.md`, steps 1–2.
- Codex through app-server: P1 step 5 (`docs/qa/0.4-walks/P1.md`).

## 2. Claude Code's own cross-session messages — YES, from the launch

Claude Code (v2.1.224 and later) lets its sessions on one Mac message
each other with its own `SendMessage` and `ListAgents` tools, over a
per-session socket that never passes through AI Juggler.

- **Before (live, 2026-11-07, P7 step 3):** a session in another pool used
  Claude's own SendMessage on c1 in the sealed Client pool. c1 received
  it and ran it: `"echo lab" to client session → sent to client-99`.
  AI Juggler's bridge never sees that channel.
- **The documented settings** (code.claude.com/docs/en/cross-session-messaging):
  - `crossSessionInbound`: `"accept"`, `"hold"` or `"refuse"`. `refuse` drops other sessions' messages.
  - `permissions.deny: ["SendMessage", "ListAgents"]` stops the session reaching others.
  - A value from `--settings` takes precedence over the user's own settings.
  - The Claude desktop app's Code tab can't show the hold dialog: a held message waits until `dialogExpiry`. `refuse` also drops the desktop surface's messages.
- **Built (57f89e1bb, a3903836e):** a Claude Code launch in a sealed pool gets both keys in a per-launch settings file in AI Juggler's overlay folder.
  - It is the launch's last step (the adapter's `sealed_launch`), and it applies whether or not the agent tools are on.
  - The file holds every `--settings` before it (the Account's overlay, the hooks), so it reads the same whether Claude merges several or keeps the last.
  - It applies from the session's next start; a running session keeps its launch's settings.
  - Other apps' sessions are Watched and aren't covered.
  - The wizard says so.
- **Live on rc2 (2026-11-07):** a team session's own SendMessage into the sealed client session was refused ("isn't accepting cross-session messages"), and the client ran nothing. The client had no ListAgents or SendMessage; AI Juggler's pool tools still worked inside the pool.
- **Pair rules other than sealed:** Ask first or Blocked between two members can't be set on Claude's channel per pair, because the setting covers the whole session. Pair rules hold on AI Juggler's channel only.

## 3. OpenCode and Grok Build tools per launch — YES at load (no sign-in); the live part is on rc17

Lab of 2026-11-09:
- OpenCode 1.19.5 and Grok Build 1.0.77, as installed.
- Every home was pointed at throwaway folders: `HOME`, the `XDG_*` folders, `OPENCODE_CONFIG_DIR`, `GROK_HOME`, and Grok's `--leader-socket`. The maintainer's own `~/.grok` and OpenCode folders were never read.
- A stub stdio MCP server stood in for `juggler-agent` (it answers `initialize` and lists one tool).

- **OpenCode: yes, per launch.** With `OPENCODE_CONFIG_CONTENT='{"mcp":{"juggler":{"type":"local","command":["python3","-I","<stub>"],"enabled":true}}}'`, `opencode mcp list` printed `✓ juggler connected`. Nothing was written, and nothing asked.
- **Grok Build: not through its overlay; yes through its home.**
  - With `GROK_CONFIG` holding `{"mcp_servers":{"juggler":{…}}}`, `grok mcp list` printed "No MCP servers configured". The overlay is allowlisted and can't add a server, as the research said.
  - With `[mcp_servers.juggler]` in `$GROK_HOME/config.toml` (`command`, `args`, `env = { KEY = "${KEY}" }`, `enabled = true`), `grok mcp list` lists it. Grok expands `${VAR}` at load, so the key stays in the launch's environment.
  - `GROK_HOME` is per Account, so AI Juggler writes this entry only into a home it made, never into the user's own `~/.grok`.
- **What the build does with it (0c9a4711a, 0.4.1):** both adapters give their launches the agent tools, through the one adapter method every tool answers. The conformance kit checks all of them: a launch gets the tools, or its debug log says why.
  - **OpenCode:** `OPENCODE_CONFIG_CONTENT` gains `mcp.juggler` (the bridge with `--mcp`, its variables by `{env:NAME}`) and `permission.juggler_<tool>: "allow"` for AI Juggler's 14 tools only. It merges with the launch's own inline settings (the model kept, a user's `"ask"` kept for every other tool). Nothing is written.
  - **Grok Build:** a home AI Juggler made keeps `[mcp_servers.juggler]` (the bridge with `--mcp`, its variables as `${NAME:-}`, `enabled = true`). Each launch passes `--allow MCPTool(juggler__<tool>)` for the 14 tools. The user's own `~/.grok`, or a home brought in place, is never written; that Account's launches get the command line only, with the reason.
  - Probe with the real bridge (throwaway homes, no sign-in):
    - OpenCode: `opencode mcp list` printed "✓ juggler connected", and `opencode debug config` showed the model kept and exactly the 14 allows.
    - Grok: `grok mcp doctor juggler` printed "handshake OK", "14 tools discovered" and "1 healthy, 0 failing", the same for a launch without the tools, where each call says the session wasn't started with them.
- **Left for the live walk** (`docs/qa/0.4-walks/E3002-R3.md`, rc17, a signed-in test account of each): an agent sees the 14 tools and calls one with no prompt; messages and holds work; and a launch without the tools shows no broken server.
- Kimi Code stays on its shell.
