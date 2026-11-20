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
- **`hold`, live in the terminal (2026-11-10, verified by Opus; evidence `docs/qa/evidence/AJ-E3002/manifest.md` §"Claude Code's own `crossSessionInbound: \"hold\"`"):** two Claude Code 2.1.310 sessions in `~/lab/web` on the Lab App folder, R with `claude --settings '{"crossSessionInbound":"hold"}'`, S plain. S's SendMessage to R was answered "Cross-session message held by the receiving session … Claude in that session hasn't seen it." R showed one line, "Held peer message … not delivered to Claude (1 held). Your "crossSessionInbound" setting is "hold"; set it to "accept" to deliver held messages.", with no dialog and no way to approve in place; after ~90 s it was still held, and Claude in R had seen nothing. So `hold` holds per session; the only release is changing the setting.
- **`hold`, live in the Claude app's Code tab (2026-11-10, verified by Opus; same manifest, §"the Claude desktop app's Code tab"):** on the sanctioned aij-test profile, its `settings.json` set to `{"crossSessionInbound":"hold"}` and restored byte for byte after (SHA-256 `ca3d163b…` before and after). A message sent to the app's session on Claude's own channel was told "held"; the Code tab showed one dashed in-line notice, "held - waiting for approval in that session's terminal … Claude has not seen it", with no dialog and no button, and the message wasn't delivered in ~5 minutes. That matches the documented "the Code tab can't show the hold dialog": a held message waits and isn't delivered, so AI Juggler offers `refuse` (sealed), never `hold`, on Claude's channel (D-95).

## 3. OpenCode and Grok Build tools per launch — YES, at load (no sign-in) and live

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
- **Live, on 0.4.1's release candidates (rc17 to rc33), 2026-11-09..11.** The walk followed the sheet `docs/qa/0.4-walks/E3002-R3.md`: AI Juggler Dev on the real HOME under ADR-0027 7b, a signed-in test account of each tool in an App AI Juggler made for it, and the pool "R3 tools" on `~/lab/r3-tools` with the Lab App as Lead. The release session's Opus-checked walk results aren't in the repository, so this is written from what they left in it: the evidence manifest (`docs/qa/evidence/AJ-E3002/manifest.md`, R3), AJ-E3304's P8 row, and the commits that fixed what the walk found.
  - **The tools, with no prompt (steps 5 and 6, rc17 and rc20):** in each session, "list the tools you have from the juggler server", then `whoami`. OpenCode and Grok Build both got AI Juggler's 14 agent tools, with no permission prompt and no approval card.
  - **Again on rc27** (P8 step 1, `docs/qa/evidence/AJ-E3304/manifest.md`): the OpenCode and Grok scouts called `send_message` and `share_doc` with no prompt.
  - **Messages and holds (step 8):** Member → Lead on Ask first, Grok sends the Lead "echo lab"; then Lead → Grok the same way. Delivered both ways after Allow once. Seen on rc19, while the message was held:
    - The rail's Needs you row read "«Untitled session · r3-tools started 01:15 PM» wants t…", with its reason cut off. Fixed in b441abbf5: "«r3-tools» wants to message «web»", with "Ask first in «Team»" under it.
    - The Messages page only said "Waits on your OK"; only the canvas card could answer. Fixed in 0bc92bbba: Allow once and Deny are on the Messages page too.
    - A line exists only after a first message, so its own rule can be set only after one got through. That is AJ-E3010, planned for 0.5.
  - **A launch without the tools (step 12):** Agent tools off for `~/lab/r3-tools`, then a Grok session.
    - On rc18, Grok still listed the juggler server its home keeps, and `whoami` raised Grok's approval card. A FAIL.
    - Fixed in 149d0e34f: such a launch passes `--deny MCPTool(juggler__*)`, and `--rules` saying the session wasn't started with AI Juggler's agent tools (Grok 1.0.77 can't leave one configured server out per launch).
    - Walked again: Grok is denied the tools and shows no card.
    - On rc32 the deny came back as Grok's own rule string. Since 13113d5b2 the `--rules` words give the reason and tell the agent to say in plain words that the tools are off and where to turn them on.
  - **Found on the other steps and fixed:**
    - rc17, step 2: OpenCode's `auth login` with a pasted key wrote an `api` entry. The sign-in row kept waiting on "Finish signing in below", and Needs you kept asking it to sign in. Fixed in 56614838e: "Signed in with an API key".
    - rc18, step 1: `grok --version` printed "grok 1.0.77 (50ca579fa996)", and Add an app said "Found; couldn't check its version". Fixed in bd431c24d.
    - rc18: an OpenCode session idle at its prompt after its turn read "Probably stuck" in the rail and the pool. Fixed in 1ab7cfc59 (each tool says its own end of turn), and in 8e48d8938 for Codex and Grok Build.
    - rc32, step 10: Resume started a new session that no member matched, and the original's node stayed "Stopped by you". Fixed in e1135059c: a resumed session keeps the pools it was put in.
    - rc33: a "Grok · R3" terminal session came back on its conversation 6 s after its Stop: once the Stop landed, Resume took its place, and the next press there resumed it. Fixed in 5b7970e0e: Stop reads "Stopping…", and the Resume in its place takes no press for 2 s.
  - **Not in the records:** the agents' own words for steps 5 and 6, and the results of steps 4, 7, 9 and 11. Steps 1, 10 and 11 are AJ-E1612's checklist (found, resume, usage) and stay open there.
- Kimi Code stays on its shell.
