# Pools, agent messaging and agent task management: prior art and fit with AI Juggler

Research date: 2026-10-21
Author: research agent (read-only on the repo; nothing installed; no credentials; no `~/.claude` reads)
Evidence rules: `docs/research/EVIDENCE-STANDARD.md`. Every external claim below carries a source, an access date, a short paraphrase and a label (**VP** = Verified-primary, **VS** = Verified-secondary, **NE** = Negative-evidence, **I** = Inferred). All sources accessed 2026-10-21. Nothing here was tested live; every "works" claim is a docs claim until the Phase 0 spike proves it on pinned versions (Claude Code 2.1.x as installed, codex-cli 0.163.0).

---

## 0. Summary

- **Recommended shape:** one local AI Juggler MCP server (`juggler-agent`, stdio, shipped in the signed bundle) that every managed session gets through *per-launch flags* (Claude `--mcp-config`, Codex `-c mcp_servers.juggler.*`), talking to the app core over a per-edition Unix socket. Agents **pull**: they call tools to read the roster, context cards, inbox and tasks. That is agent-initiated work inside the user's own session, so it sits on the passive side of ADR-0003.
- **What needs a decision:** anything that makes a session take a turn without the user (waking an idle agent on a new message, Stop-hook continuations, typing into a PTY, driving Codex `turn/start`) is app-initiated. Under SPEC §1 and ADR-0003 it may only run behind a click until the maintainer records a new decision (proposed D-26 to D-31, §3).
- **Order:** Pools (with read-only context cards) → the agent bridge (read-only MCP) → messaging (pull plus click-to-deliver) → tasks → policy-gated delivery upgrades → fleet pools.
- **Biggest finding:** Claude Code now ships native **cross-session messaging** (`ListAgents` / `SendMessage`, a per-session Unix socket inbox, loop throttling, "a message can't approve anything") and **agent teams**. AI Juggler shouldn't rebuild that for Claude-to-Claude traffic. It should add what the vendors lack: mixed-vendor pools (Codex has no native peer messaging, NE), permissions that the user sets, a durable store with audit, task management across vendors, and one UI.

---

## Part 1: prior art

### 1.1 Google A2A (Agent2Agent)

| Claim | Source | Label |
|---|---|---|
| The A2A spec is at **1.0.0**, the latest released version (earlier 0.3.0, 0.2.6, 0.1.0) | https://a2a-protocol.org/latest/specification/ | VP |
| Core objects are Agent Card, Task (server-ID'd, with a lifecycle), Message (role + Parts), Part (text/file/data), Artifact (outputs), and an optional contextId grouping | same | VP |
| Task states are submitted, working, completed, failed, canceled, rejected, input-required and auth-required. The "interrupted" states resume with a new message carrying the same task and context IDs | same | VP |
| Bindings are JSON-RPC, gRPC and HTTP+JSON/REST, plus custom bindings. Streaming and webhook push notifications are declared in the Agent Card | same | VP |
| Version 1.0 was announced by the Linux Foundation, is supported by 150+ orgs, and is governed by a multi-vendor TSC | https://linuxfoundation.org/press/a2a-protocol-surpasses-150-organizations-lands-in-major-cloud-platforms-and-sees-enterprise-production-use-in-first-year | VS |
| Discovery uses `/.well-known/agent-card.json` (legacy `/.well-known/agent.json` before 0.3) | https://aaif.io/blog/a2a-v1-0-a-builder-s-guide-part-1-discovery-tasks-and-clients ; https://apievangelist.com/2026/07/29/most-published-agent-cards-are-not-actually-a2a/ | VS |
| No stdio or local-process binding was found in the visible spec text | search log §7 | NE |

**Fit:** A2A is built for networked, opaque, server-hosted agents that publish cards and authenticate over HTTP. Neither Claude Code nor Codex speaks A2A natively (I; no A2A client support found in either vendor's docs this session). AI Juggler should **borrow the vocabulary**: Task states including `input-required`, Artifacts as the output carrier, and contextId as the thread. It should not ship an A2A endpoint in v1. A localhost A2A gateway for third-party agents could come post-1.0, and it would need a network-allowlist decision (SPEC §1 Privacy).

### 1.2 MCP (spec 2026-07-28, the current revision)

| Claim | Source | Label |
|---|---|---|
| The core is now **stateless**: the `initialize` handshake is removed, and each request carries its protocol version and capabilities in `_meta`. `server/discover` is new | https://modelcontextprotocol.io/specification/2026-07-28/changelog | VP |
| Server-to-client change notifications move to one opt-in `subscriptions/listen` stream (tools/prompts/resources list-changed, resource subscriptions). `resources/subscribe` is replaced | same | VP |
| **Multi Round-Trip Requests (MRTR)** replace server-initiated requests (`elicitation/create`, `sampling/createMessage`, `roots/list`). A server returns `resultType:"input_required"` and the client retries with `inputResponses` | same | VP |
| Tasks moved to an official extension `io.modelcontextprotocol/tasks` (poll `tasks/get`, `tasks/update`; `tasks/list` removed) | same | VP |
| Roots, Sampling and Logging are **deprecated**, with at least a 12-month window | same | VP |
| Tools are "model-controlled". Hosts **SHOULD** keep a human in the loop able to deny invocations. Servers **MUST** validate inputs, apply access control, rate-limit and sanitize outputs. Annotations from untrusted servers must be treated as untrusted | https://modelcontextprotocol.io/specification/2026-07-28/server/tools | VP |
| The spec recommends explicit, server-minted state handles passed as tool arguments, since there's no protocol session | same | VP |

**Fit:** MCP has **no peer-to-peer messaging primitive**. A server can't push a prompt into a host's model, and the spec moved away from server-initiated requests. Agent-to-agent messaging over MCP is therefore *pull* (the model calls `read_inbox`) plus host-specific push extensions (Claude's `claude/channel`, §1.3). Design the AI Juggler server for the stateless model: per-request identity from a launch-bound token, and explicit handles such as `thread_id` and `task_id`. Expect clients to still speak 2025-11-25 for a while, so support both (I).

### 1.3 Claude Code and the Claude Agent SDK

| Claim | Source | Label |
|---|---|---|
| **Cross-session messaging:** Claude lists sessions with `ListAgents` and sends with `SendMessage`. It requires v2.1.224+ on macOS and Linux and is "on with nothing to enable". Same-machine delivery uses a per-session Unix socket, "never through Anthropic servers" | https://code.claude.com/docs/en/cross-session-messaging | VP |
| An incoming peer message "can't approve anything", can't change configuration, its slash commands don't run, and the receiving session's permission prompts still fire | same | VP |
| The `crossSessionInbound` setting is `accept`, `hold` or `refuse`. With no value set, delivery follows the two sessions' permission-mode classes, and a held message expires after `dialogExpiry` (5 min). The queue caps at 100 held messages and 50 accepted | same | VP |
| Built-in loop control: per-sender rate limit, identical repeats dropped, burst refusal at the sender, about 1M-character cap. "A message loop between two sessions therefore stops on its own" | same | VP |
| An idle receiver gets a **new turn** started by Claude Code itself. `notify_when_idle` subscriptions expire after 12 h | same | VP |
| The inbox socket path is exported as `CLAUDE_CODE_MESSAGING_SOCKET`, with a token in `CLAUDE_CODE_MESSAGING_TOKEN`. Scripts and hooks may post to their *own* session. `-p` sessions bind a socket; `--bare` sessions don't | same | VP |
| Whether sessions under **different `CLAUDE_CONFIG_DIR`s** (different accounts) can list and message each other isn't stated. The docs say the socket is restricted to the OS user | same; search log §7 | NE (cross-account), I (likely works: same OS user) |
| **Agent teams** are experimental (`CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1`). There's a lead plus teammates, a shared task list (pending, in progress, completed, with dependencies, claimed under file locks) and JSON mailboxes in `~/.claude/teams/{team}/inboxes/`. "One team per session", no nesting, no resume of in-process teammates | https://code.claude.com/docs/en/agent-teams | VP |
| Team token use "scales linearly"; the docs recommend starting with 3–5 teammates. Known failures: task status lags, the lead stops early, and teammates edit the same file | same | VP |
| Task tools (`TaskCreate/Get/Update/List`) replace `TodoWrite` where task tools are on. Since v2.1.268 they are on by default only on older models (Opus 4.7, Sonnet 4.6, Haiku 4.5 and earlier) and are otherwise opt-in | https://code.claude.com/docs/en/agent-sdk/todo-tracking | VP |
| **Channels** (research preview): an MCP server declaring `experimental['claude/channel']` pushes `notifications/claude/channel` events into a session. Custom channels need `--dangerously-load-development-channels` (a full-screen warning, ignored with `-p`), and Team/Enterprise orgs must enable them. "An ungated channel is a prompt injection vector" | https://code.claude.com/docs/en/channels-reference | VP |
| `--channels` "requires Anthropic authentication through claude.ai or a Console API key" | https://code.claude.com/docs/en/cli-reference | VP |
| Flags AI Juggler can use per launch: `--mcp-config <file>` (adds servers; `--strict-mcp-config` would drop the user's), `--settings <file>` (hooks and keys override for this session), `--append-system-prompt-file`, `--allowedTools`, `--name`, and for headless runs `--input-format stream-json`, `--max-turns`, `--max-budget-usd` | https://code.claude.com/docs/en/cli-reference | VP |
| `.mcp.json` env expansion `${VAR}` / `${VAR:-default}` works in command, args and env. MCP output warns at 10k tokens and caps at 25k by default. Tool search (deferred loading) is the default. `list_changed` is honoured | https://code.claude.com/docs/en/mcp | VP |
| Hooks can return `hookSpecificOutput.additionalContext` on SessionStart, UserPromptSubmit, PreToolUse, PostToolUse, PostToolBatch, Stop and others. The docs advise stating facts rather than imperatives, because imperatives "can trigger Claude's prompt-injection defenses". Stop exit 2 continues the conversation | https://code.claude.com/docs/en/hooks | VP |
| Agent SDK: "Unless previously approved, Anthropic does not allow third party developers to offer claude.ai login or rate limits for their products, including agents built on the Claude Agent SDK." | https://code.claude.com/docs/en/agent-sdk/overview | VP |
| Legal: developers "may not ... route requests through Free, Pro, or Max plan credentials on behalf of their users", but that "does not ... prevent an end user from signing in to the unmodified Claude Code binary with their own Claude subscription". Pro/Max limits "assume ordinary, individual usage" | https://code.claude.com/docs/en/legal-and-compliance | VP |
| Consumer Terms §3.7: no access "through automated or non-human means, whether through a bot, script, or otherwise", except via an API key "or where we otherwise explicitly permit it" (effective 2025-10-08) | https://www.anthropic.com/legal/consumer-terms | VP |

**Fit:** For Claude, the vendor already sanctions agent-initiated peer messaging, wakes idle receivers, and enforces provenance and loop limits. AI Juggler should **not** post into Claude inbox sockets on its own initiative; that would be app-initiated (§2). It *may* show Claude-native traffic it can observe passively (the transcripts already indexed) and should add its own pool tools for mixed-vendor pools. The Agent SDK is off-limits for subscription accounts: it requires API-key auth for third-party products.

### 1.4 Codex: app-server, SDK, MCP, subagents

| Claim | Source | Label |
|---|---|---|
| `codex app-server`: JSON-RPC over stdio (default), Unix socket, or WebSocket (experimental). `initialize` with `clientInfo` comes first. Enterprise integrations are told to "contact OpenAI to get it added to a known clients list" | https://learn.chatgpt.com/docs/app-server.md | VP |
| Methods include `thread/start`, `resume`, `fork`, `read` and `list`, plus `turn/start`, `turn/steer` (add input to the in-flight turn) and `turn/interrupt`. Approvals are server-initiated requests | same | VP |
| Experimental **`dynamicTools`** on `thread/start`: client-supplied tools, invoked through an `item/tool/call` request | same | VP |
| Subagent threads (`subAgentThreadSpawn`, `parentThreadId`) and `collabToolCall` items exist | same | VP |
| `codex --remote unix://PATH` attaches the TUI to an app-server. `--no-daemon` "can't be combined with `--remote`". **`codex mcp-server` has been removed** | https://learn.chatgpt.com/docs/cli/reference | VP |
| `-c key=value` overrides with TOML-parsed values sit at the top of config precedence. Dotted paths such as `mcp_servers.x.enabled=false` work | same; https://learn.chatgpt.com/docs/config-file/config-advanced | VP (flag), VS (dotted mcp example) |
| MCP config lives in `[mcp_servers.<id>]` with `command`, `args`, `env`, `env_vars`, `cwd`, `required`, `enabled_tools`/`disabled_tools`, `startup_timeout_sec` (10), `tool_timeout_sec` (60), and `default_tools_approval_mode` = `auto`, `prompt`, `writes` or `approve` (`writes` prompts only for tools not marked read-only). Project `.codex/config.toml` loads only in trusted projects | https://learn.chatgpt.com/docs/extend/mcp?surface=cli | VP |
| Subagents are on by default (`agents.enabled`). Spawning is explicit (or requested by AGENTS.md or skills). Custom agents go in `.codex/agents/*.toml` and can't override `mcp_servers`. Children inherit sandbox and approvals. "The page doesn't describe direct agent-to-agent messaging" | https://learn.chatgpt.com/docs/agent-configuration/subagents.md | VP; NE for peer messaging |
| `developer_instructions` is a config key that injects developer instructions | https://learn.chatgpt.com/docs/config-file/config-reference | VP |
| Hooks: SessionStart and UserPromptSubmit accept `additionalContext`; Stop `decision:"block"` continues with the reason as a new prompt. "Non-managed hooks must be reviewed and trusted before they run" (trust is per hash) | https://learn.chatgpt.com/docs/hooks.md | VP |
| The Codex SDK (`@openai/codex-sdk`) wraps the CLI over JSONL, with `startThread()` / `run()` / `resumeThread()` | https://socket.dev/npm/package/@openai/codex-sdk/overview/0.146.0-alpha.13 | VS |
| Repo: AI Juggler already passes `--no-daemon` for every terminal launch so the app-server stays inside the owned tree (ADR-0010) | `crates/juggler-adapters/src/codex.rs` module docs | VP (local) |

**Fit:** Codex gets AI Juggler's tools through `-c mcp_servers.juggler.*` on argv, with nothing written to the user's `config.toml` (ADR-0018 §3 pattern). Codex hooks need user trust per hash, so AI Juggler must never use `--dangerously-bypass-hook-trust`. App-server (`turn/steer`, `dynamicTools`) is the strongest push route, but it's app-initiated, it conflicts with today's `--no-daemon` TUI model, and OpenAI asks integrators to register as a known client. It's a Phase 5 spike, not v1.

### 1.5 Agent mail and inbox MCP servers

| Claim | Source | Label |
|---|---|---|
| mcp_agent_mail gives agents memorable identities, inbox and outbox, threads, search and acks. The README says the Python version is no longer maintained (superseded by a Rust version) | https://github.com/Dicklesworthstone/mcp_agent_mail | VP |
| Cross-project messaging needs an explicit **contact approval** (`request_contact`/`respond_contact`). AI-suggested links are only suggestions. A "Human Overseer" bypasses contact policy | same | VP |
| **Advisory file reservations** (TTL, exclusive or shared), with an optional pre-commit guard | same | VP |
| Storage is dual: Git (Markdown/JSON audit trail) plus SQLite FTS5. The primary transport is HTTP (127.0.0.1:8765), with stdio optional. Local dev can bypass auth, and the web UI's GET pages aren't under RBAC | same | VP |
| Open issues include dependency advisories, a datetime bug and a UI refresh loop | https://github.com/Dicklesworthstone/mcp_agent_mail/issues | VP |

**Lessons:** contact consent, overseer messages and advisory leases are good ideas to copy. An HTTP server with auth bypassed for local dev is not; use a socket restricted to the OS user. Agents only see mail when they decide to check, which is the core weakness of pull-only designs (I, from the design).

### 1.6 Task systems built for agents

| Claim | Source | Label |
|---|---|---|
| **Task Master** runs as an MCP server (`npx task-master-ai`) and parses PRDs into tasks, subtasks, dependencies, complexity scores and tags. `TASK_MASTER_TOOLS` = all (36 tools, about 21k tokens), standard (15, about 10k) or core (7, about 5k). Claude Code users need no API key | https://github.com/eyaltoledano/claude-task-master | VP |
| **Beads** (`bd`): a "distributed graph issue tracker for AI agents" on Dolt. Hash IDs (`bd-a1b2`) avoid merge collisions; hierarchical IDs mark epics and tasks; links are blocks, related or parent-child. `bd ready` lists unblocked work and `bd update --claim` claims atomically. Embedded mode allows a single writer | https://github.com/steveyegge/beads | VP |
| **Backlog.md**: one Markdown file per task under `backlog/`, acceptance criteria and Definition of Done, milestones, MCP via `backlog mcp start` (workflow guides exposed as resources), and a local kanban on 127.0.0.1. Its advice: one task per session, one PR per task, the plan goes into the task, and a human approves before code | https://github.com/MrLesk/Backlog.md | VP |
| **Linear MCP** (official): `https://mcp.linear.app/mcp`, Streamable HTTP, OAuth 2.1 or API key, a `/mcp/readonly` endpoint, tools to find, create and update issues, projects and comments. Setup commands exist for Claude Code and Codex | https://linear.app/docs/mcp | VP |
| **GitHub MCP server** (official): remote and local, toolsets (`issues`, `pull_requests`, `projects`…), a `--read-only` mode, sub-issue tools | https://github.com/github/github-mcp-server | VP |
| Claude Code's Task tools and the team task list (pending, in progress, completed, dependencies, locks) | §1.3 sources | VP |

**Lessons:** (1) Tool-count context cost is real: Task Master needed lean tiers. Keep AI Juggler's surface at about 8–10 tools, rely on deferred tool search, and return short results. (2) Atomic claiming and "ready = no open blockers" are the two primitives every agent tracker converges on. (3) Hash or short IDs beat sequential IDs when several writers exist. (4) Users already keep tasks in Linear, GitHub or Backlog.md. AI Juggler tasks should *link* external issue URLs, not replace those trackers, and import/export can come later.

### 1.7 Multi-agent orchestrators and group chat

| Claim | Source | Label |
|---|---|---|
| **Claude Squad:** a terminal app; tmux plus git worktrees per agent; Claude Code, Codex, Gemini, Aider and others; experimental `-y/--autoyes` auto-accept; requires tmux and gh; no inter-agent messaging described | https://github.com/smtg-ai/claude-squad | VP |
| Critics cite the tmux dependency and hidden session status | https://speakerdeck.com/knishioka/conductor-git-worktreedeshi-xian-surubing-lie-aikodeingu | VS |
| **Conductor:** a Mac app; a worktree per workspace; Claude Code and Codex on the user's own subscriptions; setup scripts and `CONDUCTOR_PORT`; built-in review and merge | https://conductor.build/docs ; https://www.conductor.build/docs/core/parallel-agents | VP |
| Conductor criticisms: Mac-only, proprietary, requires a GitHub repo | https://superset.sh/compare/conductor-alternative (a competitor's page) | VS |
| **Vibe Kanban:** kanban issues → workspaces (branch, terminal, dev server), inline diff review, 10+ agents, an MCP server. The README says "Vibe Kanban is sunsetting"; the company cited no viable business model and the project goes community-maintained | https://github.com/BloopAI/vibe-kanban ; https://vibekanban.com/shutdown | VP |
| **Crystal:** parallel Claude Code and Codex sessions in worktrees; deprecated Feb 2026 in favour of Nimbalyst | https://github.com/stravu/crystal | VP |
| **CrewAI:** `allow_delegation=True` gives "delegate work" and "ask question" tools, plus a hierarchical manager process. To avoid delegation loops, the docs recommend one coordinator and specialists with `allow_delegation=False` | https://docs.crewai.com/en/concepts/collaboration | VP |
| **AG2/AutoGen** group chat: a group manager picks the next speaker (auto, round-robin, random or manual patterns), with `max_rounds` as the stop | https://docs.ag2.ai/latest/docs/user-guide/advanced-concepts/orchestration/group-chat/introduction/ | VP |
| Anthropic's multi-agent research write-up: agents use about 4× the tokens of chat and multi-agent systems about 15×. Early failures included "spawning 50 subagents for simple queries", endless searching and duplicated work. Coding has "fewer truly parallelizable tasks" | https://www.anthropic.com/engineering/multi-agent-research-system | VP |
| Cognition: "Share context, and share full agent traces"; "Actions carry implicit decisions, and conflicting decisions carry bad results". The default advice is a single-threaded agent | https://cognition.com/blog/dont-build-multi-agents | VP (blog, primary author) |
| Prompt Infection: LLM-to-LLM injection self-replicates and spreads faster with a global shared history. Defence: tag message origin ("LLM Tagging") | https://arxiv.org/abs/2410.07283 | VP (paper) |

**What works:** worktree isolation, review before merge, a visible status per agent, small self-contained tasks, and one coordinator.
**What users complain about:** token cost; agents editing the same files; leads stopping early or doing the work themselves; stale task status; tmux fragility; vendor lock-in and closed source; tools dying (Vibe Kanban, Crystal). No dedicated complaint threads about Conductor were found (NE, §7).

---

## Part 2: fit with AI Juggler

### 2.1 What the repo says today (local observation, 2026-10-21)

- **SPEC §1 automation invariant:** "The app *prepares*; the user *executes*." Automation is passive only (monitoring, indexing, accounting, notifications, alerts, suggestions, staging). Every state-changing action is "bound to an explicit user click". There are three recorded exceptions (auto-restart on crash, verified cleanup D-08, pack secret policy D-14), and "no others may be invented". (`docs/SPEC.md` §1)
- **ADR-0003:** the same rule, plus no login automation, no proxying, unmodified CLIs, local-only data, vendor terms surfaced, and a CI guardrail suite. The rejected alternatives were quota-driven rotation and a proxy/gateway.
- **ADR-0002:** only adapters know Provider specifics. Capabilities flags, `discover()` with `verified_against`, unknown versions degrade.
- **ADR-0010:** the spawn ledger is the ownership authority, fail-closed. `McpServer` role guesses are *hard-excluded* from automatic cleanup at every tier.
- **Supervisor:** sessions spawn under `portable-pty` (`crates/juggler-supervisor/src/supervisor.rs`, `PtyHandle {writer, master}`). The only production caller of `Supervisor::write_input` is `terminal_write_impl` in `crates/juggler-app/src/terminal.rs`, which carries the user's xterm keystrokes. **Today the app never types into a session by itself** (local grep). Programmatic first turns go in only as launch arguments on a user click: `LaunchMode::Primed {prompt}`, `Resume {delta_prompt}`, `Headless {prompt}` (`crates/juggler-adapter/src/launch.rs`).
- **Handover:** "everything here *prepares*; only an explicit user action executes" (`crates/juggler-handover/src/lib.rs`). Pack v2 has a D-14 redaction manifest, which is reusable for shared docs.
- **How launch config is written:** Claude launches add `--model` and, for in-place accounts, `--settings <overlays>/<account>/launch-settings.json` (status-line capture), written in **AI Juggler's own folder**. The code comment reads "never as edits to the folder's settings.json (ADR-0018 §3)" (`crates/juggler-adapters/src/claude_code.rs` ~l.455; `quota_ingest.rs::ensure_launch_overlay`). Codex launches add `--no-daemon` and `--model` (`codex.rs` ~l.620–660). **No MCP config is written or passed today** (grep for `mcp` in `juggler-adapters/src` finds only transcript parsing). `SettingsOverlay` holds `model` and `env` only; the SPEC sketch mentions "mcp" but the code doesn't have it yet.
- **Storage:** SQLite with WAL and numbered `MIGRATIONS` (`juggler-core/src/storage/mod.rs`): `sessions`, `projects`, `work_items`, `epochs`, `switch_edges`, `transcript_lines` with FTS5, `instance_transitions` and `instance_lifecycle` (status journal), `fleet_outbox`/`fleet_replica`. The Work item survives every switch (ADR-0011). That's the right anchor for an agent's identity in a pool.
- **Decision log:** D-01..D-25 are settled (`docs/tasks/README.md`). The next free number is **D-26**.

### 2.2 Q1: which delivery routes respect ADR-0003 and vendor terms?

Classification test: *who decides that a model turn happens, and who decides what goes into it?* If the agent (inside a turn the user started) or the user (by a click) decides, the route is compliant. If AI Juggler decides on its own, the route is app-initiated and needs a decision.

| # | Route | Who initiates | ADR-0003 | Vendor terms | Verdict |
|---|---|---|---|---|---|
| R1 | Agent calls an AI Juggler MCP tool (`read_inbox`, `send_message`, `task_claim`…) inside its own session | Agent, inside a user-started turn | Passive for AI Juggler: it stores and serves data, like indexing | MCP is the vendor-sanctioned extension point; the CLI is unmodified | **Allowed.** Needs D-26 to record that agent-written store rows aren't "state-changing actions" |
| R2 | AI Juggler injects its MCP server into launches through flags (`--mcp-config`, `-c mcp_servers.*`) | The user's launch click | Same class as today's `--settings` overlay; files stay in AI Juggler's folder | Fine; it adds a server and modifies nothing in the binary | **Allowed, needs D-28** (default on or off; whether read-only tools are pre-allowed) |
| R3 | A hook (Claude `--settings` hooks; Codex hooks need user trust) adds "2 unread messages from X" as `additionalContext` at the next turn boundary | The agent's turn is already running; the hook only annotates | Passive (a notification inside an active turn) | Vendor hook mechanism | **Allowed** for the notice text only. Never a Stop-hook "continue" (that's R7) |
| R4 | The user clicks **Deliver** on a staged message; AI Juggler types it into the PTY (or launches `Resume{delta_prompt}`) | The user | Exactly the "prepare / execute" model | The user's own input | **Allowed** (today's pattern) |
| R5 | Claude-native `SendMessage` between Claude sessions, started by Claude itself | Agent (vendor feature) | AI Juggler isn't involved; it may *display* traffic passively | Vendor-built and vendor-gated | **Allowed.** AI Juggler must not change the user's `crossSessionInbound` without a click |
| R6 | AI Juggler **wakes an idle session** when a message arrives (PTY keystrokes, posting into the Claude inbox socket, Claude channel push, Codex `turn/start`) | **AI Juggler** | **Violates** §1 unless a new amendment is recorded | Grey: Consumer Terms §3.7 "automated or non-human means"; Claude's legal page says limits "assume ordinary, individual usage"; dev channels need a dangerous flag; Codex app-server asks clients to register | **Needs D-27.** Default: no; click-to-deliver only |
| R7 | A Stop hook with `decision:"block"` (Codex) or exit 2 (Claude) keeps the agent working while unread mail or tasks exist | AI Juggler's hook | App-initiated continuation, an automatic extra turn | Same grey zone as R6, plus loop risk | **Needs D-27.** Recommend: never by default |
| R8 | An agent **assigns** a task to another pool member | Agent (R1) | Writing a row is passive, *unless* it causes a launch or a wake | Fine | **Allowed** as data. Anything that would *launch* a session is staged for a click (D-29) |
| R9 | An agent asks AI Juggler to **spawn** a new session or account to take work | Agent asks, AI Juggler would act | A launch is state-changing | Choosing an account by remaining quota is the brightest ToS line (landscape §3f.2) | **Stage only, with a user click (D-29).** Never quota-aware routing |
| R10 | Driving sessions through the Agent SDK with subscription logins | AI Juggler | n/a | **Prohibited**: "does not allow third party developers to offer claude.ai login or rate limits" | **Never** |
| R11 | Codex app-server owned by AI Juggler (`--remote unix://`) for `turn/steer` and `dynamicTools` | Depends on use | Read-only use is passive; `turn/*` is app-initiated | Experimental API; "known clients list" | **Spike only (Phase 5), D-27 governs turns** |
| R12 | Pools spanning machines over fleet gRPC/mTLS (juggler-link) | Agent (R1), relayed | Passive relay | No vendor traffic | **Needs D-30** (message content leaving the Mac; privacy invariant) |

**Honest gap:** pull-only delivery (R1+R3+R4) means an idle agent never sees a message until the user acts. That's the main product cost of staying compliant. The best mitigation is a **Needs-you row** in the existing attention model (AJ-E1004 "needs you" plus notification): "api-worker has 1 message; Deliver". The user has one click, and AI Juggler has done the preparation, which matches ADR-0003's consequence that "the 'one click' is genuinely one click".

### 2.3 Q2: proposed architecture

```
 Claude Code (PTY)                      Codex TUI (PTY, --no-daemon)
   │ --mcp-config <ov>/s123/mcp.json      │ -c mcp_servers.juggler.command=".../juggler-agent"
   │ --settings  <ov>/s123/settings.json  │ -c mcp_servers.juggler.args=["--mcp"]
   ▼ (spawns stdio child)                 ▼ -c mcp_servers.juggler.env_vars=["JUGGLER_AGENT_TOKEN"]
 juggler-agent --mcp  ◄── env JUGGLER_AGENT_TOKEN (set in LaunchSpec.env, per session) ──►
   │  UDS: <edition data dir>/agent.sock  (0600, OS-user only; edition-scoped per D-23)
   ▼
 AI Juggler core: AgentBridge service ──► SQLite (pools, members, grants, messages, docs, tasks, events)
   ▲                                    └► read model: sessions, work_items, instance_transitions, transcript IR
   └── Tauri UI: Pools · Inbox · Tasks (reuse rows / headers / Inspector tabs)
```

**Components**

1. **`juggler-agent` binary** (new crate `juggler-agentbridge`; ships signed inside the bundle). With `--mcp` it's an MCP stdio server; without it, a CLI (`juggler-agent inbox`, `send`, `tasks`) so any tool with a shell can participate (degradation path). It holds no state: every call is forwarded over the socket with the token. It exits when stdin closes. As an `McpServer`-role descendant it's already excluded from automatic cleanup (ADR-0010).
2. **Identity binding:** at launch, AI Juggler mints a random per-epoch token and records `token → (session, work_item, pool memberships)` in the ledger row's context. The agent can't claim another identity. Rotate the token on every new epoch (switch or resume), and keep membership on the **Work item**, so a switch to another account keeps the agent's inbox and tasks (ADR-0011 fit). Keep the token out of argv: Claude reads it through `${JUGGLER_AGENT_TOKEN}` expansion in the per-session `mcp.json`; Codex reads it through `env_vars` forwarding (VP for both mechanisms; behaviour still to prove in Phase 0).
3. **AgentBridge service** in the core: enforces grants, rate limits, size caps and hop counts; writes append-only events; emits UI events.

**Pools model**

- `pool`: id, name, kind (`default` | `user`), project scope (optional), created_by=user. **The default pool "All sessions" always exists**, can't be deleted, and members are added automatically on launch. Its policy is set by D-31 (proposed: the same Project only, and accounts of one *identity group*; see the leakage row in §2.5).
- `pool_member`: pool_id, member_ref (`work_item:<id>`, falling back to `session:<id>`), display name (from session `--name` / branch / folder per AJ-E1017), role (`lead` | `member` | `observer`), joined_at, left_at. The **user is always the implicit Owner** and isn't a row.
- **Grants (per pool, user-edited only, never through MCP):**

| Capability | Owner (user) | Lead | Member | Observer |
|---|---|---|---|---|
| See roster and context cards | ✓ | ✓ | ✓ | ✓ |
| Read own inbox | ✓ | ✓ | ✓ | ✓ |
| Read others' messages | ✓ (UI) | – | – | – |
| Message one member | ✓ | ✓ | ✓ (default) / lead-only (option) | – |
| Broadcast to the pool | ✓ | ✓ | – | – |
| Share a doc | ✓ | ✓ | ✓ | – |
| Create task / story / epic | ✓ | ✓ | task only | – |
| Assign to someone else | ✓ | ✓ | – | – |
| Claim an unassigned task | ✓ | ✓ | ✓ | – |
| Update own task status | ✓ | ✓ | ✓ | – |
| Close someone else's task | ✓ | ✓ | – | – |
| Cross-pool message | only through a user-approved pool link (the mcp_agent_mail contact pattern) | | | |

**Message store (SQLite migration)**

```
messages(id TEXT PK, pool_id, thread_id, sender_ref, sender_kind ('agent'|'user'|'system'),
         kind ('note'|'question'|'answer'|'handoff'|'review_request'|'status'),
         subject, body, body_sha256, hop INTEGER, reply_to, created_at, size_bytes)
message_recipients(message_id, recipient_ref, state ('queued'|'notified'|'read'|'acked'|'held'|'refused'|'expired'),
                   state_at, delivered_via ('pull'|'hook_notice'|'user_click'))
docs(id, pool_id, sha256, mime, size, blob_path, redaction_manifest_json, created_by, created_at)
pool_links(a_pool, b_pool, approved_by_user_at, revoked_at)
agent_events(id, at, actor_ref, verb, object_ref, detail_json)   -- append-only audit (S-G1 style)
```

FTS5 over subject and body reuses the existing tokenizer setup. Doc blobs live in the edition data folder, content-addressed. Before a doc is stored, it goes through the D-14 redaction pipeline, with the user's remembered choice shown, the same as packs.

**Context cards ("what each agent is working on")**

Built from the read model already in the repo, honest fields only (D-06/D-07):

| Field | Source | If missing |
|---|---|---|
| Name, place | session `--name`, branch, else folder (AJ-E1017) | – |
| Provider · status | `sessions.status`, `instance_transitions` (Running/WaitingUser/Stuck/QuotaBlocked…) | – |
| Current task | `tasks.assignee = member` and `status = in_progress` | "No task claimed" |
| Self-reported line | last `set_status(text)` (≤140 chars, marked "said by the agent") | "Unavailable — agent hasn't reported" |
| Last activity | `last_activity` | – |
| Files touched recently | transcript IR edit items | "Unavailable — Provider doesn't expose edits" |
| Account | hidden by default on cards an agent can read (privacy; D-31) | – |

Cards are served as a tool result and a resource (`juggler://pool/{id}/cards`), and shown in the UI as rows in the existing Home/rail patterns.

**Tasks**

```
tasks(id TEXT PK  -- short hash e.g. 'aj-7f3c' (Beads lesson), pool_id, kind ('epic'|'story'|'task'),
      parent_id, title, body, acceptance_md, status ('todo'|'ready'|'in_progress'|'blocked'|'review'|'done'|'canceled'),
      priority, assignee_ref, created_by_ref, created_at, updated_at, external_url)
task_links(from_id, to_id, kind ('blocks'|'relates'|'duplicates'))
task_sessions(task_id, work_item_id, epoch_id, linked_at)   -- links tasks ↔ Switch Graph
task_events(...)  -- append-only; also feeds agent_events
```

- `ready` = no open `blocks` predecessors. `claim` is one transaction (`UPDATE … WHERE assignee IS NULL`), which is atomic.
- MCP tools (lean): `whoami`, `pool_roster`, `set_status`, `send_message`, `read_inbox`, `ack`, `share_doc`, `tasks_list`, `task_get`, `task_create`, `task_claim`, `task_update`. That's 12, with read-only tools annotated `readOnlyHint` so Codex `writes` approval mode and Claude's `--allowedTools` can pre-allow reads only. Add `task_assign` only for leads.
- The UI follows progressive disclosure: a task list per pool first, the epic→story tree on demand, a board view later. Tasks link to the Switch Graph, so "continue on another account" carries the task.

**How each tool gets it**

| Tool | Tools | Notice at turn boundary | Instructions | Approvals |
|---|---|---|---|---|
| Claude Code | `--mcp-config <ov>/<session>/juggler-mcp.json` (never `--strict-mcp-config`, so the user's own servers stay) | `--settings <ov>/<session>/settings.json` adds a `UserPromptSubmit` + `PostToolBatch` command hook running `juggler-agent notice`, which returns facts-only `additionalContext`. The existing overlay settings file needs merging (one file per launch) | MCP server `instructions` (Claude Code delivers server instructions as context: VP for channel servers; to verify for plain servers) | Optional `--allowedTools mcp__juggler__whoami …` for read-only tools only (D-28) |
| Codex | `-c mcp_servers.juggler.command="…"`, `.args=["--mcp"]`, `.env_vars=["JUGGLER_AGENT_TOKEN"]`, `.default_tools_approval_mode="writes"` | Not in v1: Codex hooks need per-hash user trust. The "set up notices" wizard lets the *user* add and trust a hook | `-c developer_instructions="…"` is possible, but it is prompt injection by the app; prefer MCP server instructions if Codex honours them (unknown) | `writes` mode prompts for non-read-only tools |
| ZCode and future tools without MCP | `juggler-agent` CLI on PATH inside the session env (agent-initiated via its shell), plus a copyable snippet the user can paste into their AGENTS.md | – | User-pasted | Tool's own |
| Any tool, idle | Click-to-deliver (R4): the staged message is typed into the PTY on the user's click, or a `Resume{delta_prompt}` launch | – | – | – |

Adapter shape (ADR-0002): add `Capabilities::agent_bridge: {mcp_injection, hook_notice, native_peer_messaging}` and `fn agent_bridge_launch_args(&self, ctx) -> Vec<String>` behind `discover()` / `verified_against`. Unknown versions get the CLI fallback only.

### 2.4 Q3: phasing into shippable releases

| Release | Scope | Size | Done-proof ideas |
|---|---|---|---|
| **P0: Decisions + spike** | ADR-0019 "Agent bridge" + D-26..D-31 decided. Spike on sacrificial accounts (maintainer-approved per the evidence standard): `--mcp-config` server loads in Claude without an approval prompt; `-c mcp_servers.*` + `env_vars` in codex 0.163.0; Claude cross-session messaging across two `CLAUDE_CONFIG_DIR`s; hook `additionalContext` from a `--settings` file. Captures go into `docs/research/evidence/` | S | Evidence files with VP labels; staleness script passes |
| **P1: Pools** | `pools`/`pool_members` migration; default pool; create, rename and move sessions in the UI (reuse sidebar groups and rows); pool filter on Home; read-only **context cards** in the UI only (no agent access yet) | M | vitest + Playwright; a11y gate (memory rule); a guardrail test that pool operations never call `spawn`/`write_input`; maintainer hand-walk as a multi-subscription user |
| **P2: Agent bridge (read-only)** | `juggler-agent` binary; socket; token binding; injection flags in both adapters (opt-in per Launch settings or default per D-28); tools `whoami`, `pool_roster`, `set_status`; the card resource | M | Integration test: `fake_harness` acting as an MCP client through the real binary; argv assertions (no token in argv; no writes under `state_dir`), extending `tests/in_place_accounts.rs`; ledger shows the helper as `McpServer` and excluded; live run on sacrificial accounts |
| **P3: Messaging (pull)** | Messages, docs (D-14 redaction), grants UI, Inbox pane with user compose (overseer), hook notice for Claude, **click-to-deliver** for idle sessions, "needs you" rows and notifications; loop limits | M–L | Guardrail suite: there's no code path from `send_message` to `write_input` without a UI command; a ping-pong test with two fake agents stops at the hop cap and rate limit; an injection test (a message telling the receiver to change grants or approve something has no effect; provenance tag present); a size-cap test |
| **P4: Tasks** | Epics, stories, tasks, links, claims, `task_sessions` links to the Switch Graph, task list UI then tree, MCP task tools; optional external-URL links to Linear/GitHub | L | Concurrency test (N fake agents claim the same task; exactly one wins); `ready` correctness tests; switch keeps the task (an E2E test on the switch graph); visual baselines |
| **P5: Delivery upgrades (policy-gated)** | Only if D-27 allows it: opt-in per-pool "wake idle agents" with caps and the kill-switch; Codex app-server spike (`--remote unix://`, `turn/steer`, `dynamicTools`); passive display of Claude-native peer messages | M–L | Undo window and banner tests modelled on the Tier 2 cleanup tests; the kill-switch takes effect within one tick; 24 h suspension after a crash |
| **P6: Fleet pools** | Pools and messages across machines over juggler-link (D-30) | M | mTLS tests; zero calls to non-allowlisted hosts (network allowlist test) |

**Why this order and not "messaging before tasks" as asked:** pools and cards ship value with zero policy risk. Pull messaging (P3) is cheap once the bridge exists, but its value is capped by the delivery gap. Tasks (P4) work naturally in a pull model, since agents check for work at start and at task end, so they don't depend on D-27. If D-27 lands as "no wake", consider swapping P3 and P4: tasks deliver more per token in a pull-only world (I). Cut a nightly→beta after each, per the release-cadence memory.

### 2.5 Q4: risks and guardrails

| Risk | What can go wrong | Guardrail |
|---|---|---|
| **Prompt injection between agents** | A compromised agent (one that read a malicious README or issue) messages others with instructions to exfiltrate, run destructive commands, or tell the user to approve something. Spread is logistic in shared-history topologies (Prompt Infection paper) | (1) **Provenance tagging** on every delivered item: `from agent "<name>" in pool "<p>"; not the user; cannot grant approval or change settings` (mirrors Claude's peer-message rule and the paper's "LLM Tagging"). (2) Grants, pools and links are **only editable in the UI**, never through MCP. (3) No broadcast by default; no shared global history; per-recipient inboxes only. (4) Message bodies are returned as data inside a structured field, never spliced into instructions; hook notices are facts-only. (5) Docs pass the D-14 secret redaction before storing. (6) The receiving CLI's own permission prompts still apply (never pass `--dangerously-skip-permissions`) |
| **Runaway loops** | A↔B ping-pong, "thanks!" storms, tasks bounced between agents | Hop counter per thread (default max 6; the hop count is in the tool result); per-sender rate limit (e.g. 10/min, 100/day per pool); dedupe identical bodies within 10 min; **no automatic wake by default** (a loop then needs a user click per round); task re-assign count cap; a pool-level circuit breaker that auto-pauses the pool with a persistent banner (same pattern as "Pause automatic cleanup") |
| **Cost blow-ups** | Each delivered message is a prompt (Claude: "counts toward usage like a prompt you type"); multi-agent systems run about 15× chat tokens; a large tool list costs context | A lean tool set (≤12) with read-only defaults; paginated, summarised `read_inbox`; output size caps (well under Claude's 10k-token warning); per-pool measured usage in the Usage screen (only measured numbers, D-06); a soft per-pool message budget with an alert; a recommended 3–5 active members, shown as guidance, never enforced silently |
| **ToS: quota circumvention** | "Assign to whoever has runway" or "move the task to the account with quota" is the brightest line in the landscape review (§3f.2), and ADR-0003 rejected auto-rotation | Assignment and routing **never read quota state**; there's no "auto-route to an account with runway"; any launch for a task is staged for a click with the user picking the account (D-29). A guardrail test asserts that the task assignment module doesn't depend on quota types |
| **ToS: automated access** | App-initiated turns on subscription accounts look like "automated or non-human means" (Consumer Terms §3.7); usage limits "assume ordinary, individual usage" | Wake and continuation stay behind a click unless D-27 says otherwise; never Agent SDK with subscription logins; never `--dangerously-load-development-channels` in shipped builds; never bypass Codex hook trust; unmodified CLIs only |
| **Cross-account / cross-org data leakage** | A Work (Enterprise) account session and a Personal account session in the same default pool exchange code or secrets across an org boundary | D-31: the default pool is scoped to one Project *and* one identity group; cross-identity membership needs an explicit user move with a one-line warning; cards hide account details from agents |
| **File conflicts** | Two members edit the same files (a top complaint about agent teams) | Show "files touched recently" on cards; later, advisory file claims (the mcp_agent_mail lease pattern) as a task field, advisory only |
| **Vendor drift** | Channels, agent teams, cross-session messaging and the app-server are all young or experimental | Capability probing with `verified_against`; unknown versions fall back to the CLI shim; research re-verify sweep per release candidate |

---

## 3. Policy decisions the maintainer must make (proposed numbering)

- **D-26 (agent-initiated store writes are passive):** an agent calling AI Juggler MCP tools in its own session (messages, statuses, task rows) is recorded as passive data handling, not an app-executed state change. *Recommend: accept.*
- **D-27 (waking agents):** may AI Juggler start a turn in a session without a click (PTY typing, Claude inbox-socket post, channel push, Codex `turn/start`, Stop-hook continuation)? Options: (a) never, click-to-deliver only; (b) a fourth recorded amendment, opt-in per pool, off by default, with hop and rate caps, a kill-switch and an audit; (c) only through vendor-native routes the vendor gates (Claude cross-session messaging). *Recommend (a) for v1; revisit (b) after beta data.*
- **D-28 (injection default and pre-allowed tools):** is the bridge injected into every launch by default or opt-in per Launch settings, and may read-only tools be pre-allowed (`--allowedTools`, Codex `writes`)? *Recommend: opt-in at first (a first-run question once pools exist), then default on; pre-allow read-only only.*
- **D-29 (agents never launch):** a task assignment or an agent request may *stage* a launch but never execute one, and account choice is the user's, never quota-driven. *Recommend: accept as a hard rule with a guardrail test.*
- **D-30 (fleet pools):** may message and task content travel between the user's machines over juggler-link? *Recommend: later, opt-in per pool.*
- **D-31 (default pool scope and identity separation):** default pool per Project and identity group; what agents may see about accounts. *Recommend: per Project, no account details to agents.*

---

## 4. Open questions for the spike (all currently I or NE)

1. Does a Claude session in `CLAUDE_CONFIG_DIR=A` appear in `/list-agents` of a session in `B`? (NE; would make cross-account Claude messaging native.)
2. Does `--mcp-config` from a file outside the project need an approval dialog? (I: no, only `.mcp.json` project scope is documented as prompting.)
3. Does Codex honour MCP server `instructions`? Does `-c mcp_servers.juggler.env_vars=[…]` forward the token from the TUI's env? (VP on key existence, untested.)
4. Can Claude `--settings` add hooks alongside the existing status-line overlay without masking the user's hooks? (Hook merging across levels is VP; `--settings` hook addition isn't stated.)
5. What MCP protocol version do Claude 2.1.x and Codex 0.163.0 send (2025-11-25 vs 2026-07-28)?

---

## 5. Search log (for Negative-evidence claims)

All on 2026-10-21.

| Claim (NE) | Queries / sources checked |
|---|---|
| A2A has no stdio or local binding | Fetched https://a2a-protocol.org/latest/specification/ (first 100k chars of 256k); search "A2A specification /.well-known/agent-card.json agent card discovery 1.0". Not found by this search |
| Codex has no direct agent-to-agent messaging between separate sessions | Fetched https://learn.chatgpt.com/docs/agent-configuration/subagents.md, https://learn.chatgpt.com/docs/app-server.md (first 100k), https://learn.chatgpt.com/docs/cli/reference; search "Codex CLI multi-agent subagents spawn_agent config". Only parent↔subagent orchestration and `collabToolCall` items found; no peer messaging between independent sessions found by this search |
| Claude cross-session messaging across different `CLAUDE_CONFIG_DIR`s | Fetched https://code.claude.com/docs/en/cross-session-messaging (full). Not stated either way |
| User complaints about Conductor | Search "Claude Squad smtg-ai tmux worktrees …; Conductor app parallel Claude Code agents complaints"; search "Conductor docs conductor.build …". Only marketing and competitor pages found; no dedicated complaint threads found by this search |
| Agent teams "teammates ignore messages" complaints | Search "Claude Code agent teams problems token cost teammates ignore messages reddit hacker news experience". Only blog guides, no community threads found by this search |
| A2A support in Claude Code or Codex | Not found in any vendor page fetched above |

## 6. Sources (all accessed 2026-10-21)

Primary (VP): a2a-protocol.org/latest/specification · modelcontextprotocol.io/specification/2026-07-28/changelog · modelcontextprotocol.io/specification/2026-07-28/server/tools · code.claude.com/docs/en/cross-session-messaging · code.claude.com/docs/en/agent-teams · code.claude.com/docs/en/channels-reference · code.claude.com/docs/en/cli-reference · code.claude.com/docs/en/mcp · code.claude.com/docs/en/hooks · code.claude.com/docs/en/agent-sdk/overview · code.claude.com/docs/en/agent-sdk/todo-tracking · code.claude.com/docs/en/legal-and-compliance · anthropic.com/legal/consumer-terms · learn.chatgpt.com/docs/app-server.md · learn.chatgpt.com/docs/cli/reference · learn.chatgpt.com/docs/extend/mcp?surface=cli · learn.chatgpt.com/docs/config-file/config-reference · learn.chatgpt.com/docs/agent-configuration/subagents.md · learn.chatgpt.com/docs/hooks.md · github.com/Dicklesworthstone/mcp_agent_mail (+ /issues) · github.com/steveyegge/beads · github.com/eyaltoledano/claude-task-master · github.com/MrLesk/Backlog.md · linear.app/docs/mcp · github.com/github/github-mcp-server · github.com/smtg-ai/claude-squad · conductor.build/docs · github.com/BloopAI/vibe-kanban · vibekanban.com/shutdown · github.com/stravu/crystal · docs.crewai.com/en/concepts/collaboration · docs.ag2.ai/…/group-chat/introduction · anthropic.com/engineering/multi-agent-research-system · cognition.com/blog/dont-build-multi-agents · arxiv.org/abs/2410.07283

Secondary (VS): linuxfoundation.org A2A press release · aaif.io A2A v1.0 builder's guide · apievangelist.com agent-card survey · socket.dev @openai/codex-sdk README mirror · learn.chatgpt.com config-advanced (dotted `-c` example via search) · speakerdeck.com (Claude Squad / Conductor comparison) · superset.sh Conductor comparison

Local (repo, read 2026-10-21): docs/SPEC.md §1–3 · docs/adr/0002, 0003, 0010, 0018 · docs/research/landscape-and-stack.md §3 · docs/research/claude-code.md · docs/research/codex.md · crates/juggler-supervisor/src/{lib,supervisor}.rs · crates/juggler-app/src/terminal.rs · crates/juggler-adapter/src/launch.rs · crates/juggler-adapters/src/{claude_code,codex,quota_ingest}.rs · crates/juggler-handover/src/lib.rs · crates/juggler-core/src/{model.rs,storage/mod.rs} · docs/tasks/README.md (D-01..D-25)
