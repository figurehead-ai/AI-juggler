# ADR-0020: The agent bridge — `juggler-agent`, a per-edition socket, a per-epoch key

- Status: Accepted for the read-only bridge (AJ-E1704, 2026-10-25). The
  live spike on spare accounts (AJ-E1701 Req.2) is the maintainer's and
  still open: the points it must confirm are marked **unverified** below.
- Date: 2026-10-25
- Deciders: the maintainer (D-28 to D-33, 2026-10-24), agents (AJ-E1704)
- Relates to: ADR-0002 (adapters), ADR-0003 (automation policy),
  ADR-0010 (spawn ledger), ADR-0018 (per-launch files in AI Juggler's
  folder), ADR-0019 (tools as data), D-28, D-30, D-33, D-37;
  SECURITY-GUIDELINES C7; AJ-E2002 (the bridge's security design)

## Context

Agents in AI Juggler's sessions should be able to ask who they are and
who's in their pool, across tools (Claude Code, Codex, and later tools
without MCP). The pools, their access levels and the one decision point
`allowed(pool, action)` exist (AJ-E1702); the context cards exist
(AJ-E1703). Research: `docs/research/pools-messaging-tasks.md` §2.3.

## Decision

1. **A program, not a server in the app.** `juggler-agent` (crate
   `juggler-agentbridge`, `unsafe_code = "forbid"`, depending on nothing
   of ours) ships in the bundle beside the app's executable. With `--mcp`
   it is an MCP server on stdio; without it, a plain command line
   (`whoami`, `roster`, `cards`, `status <text>`) for tools without MCP. It
   holds no state: each call is one request line to the app and one
   answer line back.
2. **MCP by hand.** Newline-delimited JSON-RPC 2.0, about 300 lines,
   unit-tested: the `initialize` handshake (2024-11-05 to 2025-11-25,
   echoing a known version) and the stateless 2026-07-28 revision
   (`server/discover`; `_meta` accepted and not needed). No MCP SDK: no
   new dependency to audit (serde and serde_json are already the
   workspace's), and nothing in it the bridge doesn't use.
3. **One socket per edition**, in the app's own data folder:
   `<data>/agent/bridge.sock`, the folder `0700`, the socket `0600`. Every
   connection's peer user is checked (`getpeereid` through tokio's
   `peer_cred`); one request per connection, at most 8 KiB, read within
   2 s, at most 16 at once. A stale socket is removed only when `lstat`
   shows it is a socket of ours. No TCP or HTTP listener. Served lazily
   (the first launch that gets the bridge, or the app's start when
   sessions still hold keys).
4. **Identity is a per-epoch key.** Each launch AI Juggler starts with the
   bridge on gets 32 random bytes (hex) in its **environment**
   (`JUGGLER_AGENT_TOKEN`), with the socket's path (`JUGGLER_AGENT_SOCKET`)
   and the program's (`JUGGLER_AGENT`). Never in argv, never in a file. A
   switch or a resume is a new launch, so a new key; the session's older
   keys stop at once, and a key of a session that ended stops working.
   The app keeps only each key's SHA-256 with its session, in memory and
   in a `0600` file in the `0700` folder (so sessions that outlive a
   restart keep their tools). The bridge reads its own environment; the
   app never reads another process's (D-37): it wrote the key into the
   launch. The bridge sends the key only to a socket in a folder no one
   else can write, owned by the same user.
5. **Injection through the adapters' launch specs** (D-30, ADR-0019): the
   app hands each adapter a `BridgeLaunch` (program, server name, its
   tools, the variables' *names*, and the launch's folder in
   `overlays/agent/<session>/`); the adapter returns options to insert and
   files to write (`HarnessAdapter::agent_bridge`, declared by
   `Capabilities::AGENT_BRIDGE`). The conformance kit holds every adapter
   to it (no value from the environment, files by plain name only in the
   launch's folder, nothing pointing into the account's folder, the
   launch's arguments kept in order, no wildcard).
   - Claude Code: `--mcp-config <launch folder>/juggler-mcp.json`
     (its `env` is `${JUGGLER_AGENT_TOKEN}`, `${JUGGLER_AGENT_SOCKET}`:
     filled by Claude Code from its own environment), never
     `--strict-mcp-config`; `--allowedTools` for
     `mcp__juggler__{whoami,pool_roster,set_status}` only. Both after the
     launch's other arguments (they take several values).
   - Codex: `-c mcp_servers.juggler.command=…`, `.args=["--mcp"]`,
     `.env_vars=[…names…]`, `.enabled_tools=[the three]`,
     `.default_tools_approval_mode="prompt"` and
     `.tools.<tool>.approval_mode="approve"` for each of the three, before
     the `--` that ends its options. (E17's text said `writes`; D-30
     rejected pre-allowing read-only tools only, and `writes` would ask
     about `set_status`.)
   - Tools without MCP by launch option: the environment only (the
     command line from their shell).
   On after the one-time question (D-30), off per project or profile in
   Settings › Agent access; a launch that can't get the bridge (no
   program, no socket, a file that can't be written) starts without it,
   never fails.
6. **Every call goes through `allowed(pool, action)`** (D-28), which
   decides and audits: `whoami` and `pool_roster` are `read_roster`, the
   cards resource `read_cards`, `set_status` `set_status`. Before that,
   in order: the request's one shape (`deny_unknown_fields`: no field
   names a pool or another identity), the key, a rate limit (30 calls in
   10 s per session), the switches. An ask is a refusal that says the
   user decides, until E18 brings asks agents can wait on.
7. **What agents see** (D-33): names, places, tools, states, roles,
   profile labels (never an email, a plan, an id or a folder), and what
   each agent said of itself, marked as its own words (`says`, ≤ 140
   characters, one line, control characters removed). The status line
   is kept per member (Work item), in memory.
8. **The ledger** knows the program by name as an MCP server owned by its
   session (`guess_role`), so it is never cleaned up automatically
   (ADR-0010's tiers exclude `McpServer`); it exits when its session
   closes its stdin.
9. **Signing:** the release workflow builds it for both chips, `lipo`s
   it, and makes it Tauri's `externalBin` (`Contents/MacOS/juggler-agent`);
   `sign_macos.sh` names it as the bundle's one helper, signs it on its
   own (hardened runtime, no entitlements) before the app, and refuses a
   bundle without it.

## STRIDE (SECURITY-GUIDELINES C7)

| Threat | Where | Mitigation | Test |
|---|---|---|---|
| **S**poofing: another local process poses as an agent | the socket | `0700` folder, `0600` socket, peer uid = ours, then a 256-bit key only the launched session has; a key is checked by its SHA-256, never stored | `the_socket_is_0600_in_a_0700_folder_and_answers_only_current_keys` |
| Spoofing: an agent claims another session or pool | the request | identity is the key alone; the request has no field for a pool or a session (`deny_unknown_fields`) | `each_level_refuses_…` (the "sneaky" request), `a_request_round_trips_and_refuses_strangers` |
| **T**ampering: a launch file changed to point elsewhere | `overlays/agent/<session>/` | written `0600` in `0700` folders that must be ours; the file holds no secret | `the_key_is_in_the_environment_only_…` |
| **R**epudiation | every call | `allowed()` writes the audit (pool, actor, action, decision) for every call that reaches a decision | `each_level_refuses_…_every_call_is_audited` |
| **I**nformation disclosure: the key leaks | argv, files, logs | environment only; only its hash on disk; the ledger records argv, which never has it | `the_key_is_in_the_environment_only_…`, `the_command_line_reads_the_key_from_its_environment_only` |
| Information disclosure: account details reach agents | roster, cards | labels only (D-33); no email, plan, id or folder | `agents_see_account_labels_never_emails_or_plans` |
| **D**enial of service: call storms | the socket | 16 at once, 8 KiB, 2 s to send, 30 calls / 10 s per session | (design; the rate limit's refusal is `Limit`) |
| **E**levation: an agent changes pools, levels or rules, starts a session, gets a secret | calls | the catalog has no such action; the code an agent reaches names none of it | `what_agents_reach_has_no_path_to_a_session_a_setting_or_a_secret` |
| Elevation: a confused deputy through another agent's words | `says` | marked as the agent's own words, data not instructions; one line, ≤ 140 characters | `a_status_is_one_short_line` |

## Unverified until the spike (AJ-E1701, the maintainer's)

- That a server from `--mcp-config` loads in Claude Code without an
  approval prompt, and that `--allowedTools` pre-allows its tools.
- That Codex's `-c mcp_servers.*` with `env_vars` passes the key, and that
  `tools.<tool>.approval_mode="approve"` pre-allows each tool.
- Which MCP revision each tool sends (both lines are answered).

## Alternatives considered

- **An MCP SDK** (`rmcp`): more code than the bridge uses, a new
  dependency tree to audit; rejected for now.
- **An HTTP server in the app** (MCP's streamable HTTP): a listener any
  local process can reach, `Origin` checks, a bearer token; C7 prefers
  stdio and a Unix socket.
- **The key in argv or in the `--mcp-config` file**: visible to every
  process (`ps`) or on disk; rejected.
- **Reading the key back from the session's environment**: D-37 forbids
  reading another process's environment.
