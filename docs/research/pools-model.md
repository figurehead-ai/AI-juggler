# Pools from real use: what people run, what each tool can do, and the model (AJ-E3001)

Research date: 2026-10-30
Author: research agent (read-only on the repo; nothing installed, nothing signed in, no `~/.claude` or `~/.codex` reads)
Evidence rules: `docs/research/EVIDENCE-STANDARD.md`. Labels: **VP** verified-primary, **VS** verified-secondary, **NE** negative evidence (search log in §8), **I** inferred. **VP (lab)** means the 2026-10-26 lab (`docs/research/pools-messaging-tasks.md`, "Lab, 2026-10-26"). All web sources were read on 2026-10-30 unless a row says otherwise.
The proposal for the maintainer, with wireframes, is a separate page (the proposal HTML handed over with this task). This file holds the evidence and the reasoning behind it.

---

## 0. Summary

- **Why 0.3's pools fall short, in the code.**
  - A pool has at most one project (`crates/juggler-core/src/storage/pools.rs:50`).
  - A member is one session or Work item, and only one active row per member exists across all pools (`pools.rs:70-78`, the unique index on `member_ref`). So a session is in exactly one pool.
  - The add list is AI Juggler's own live sessions that pass `cannotJoin` (`ui/src/pages/PoolPage.tsx:118-121`, `ui/src/pages/pool/actions.ts:24-28`). Sessions other apps run are "conversations" with no session id (`ui/src/lib/conversations.tsx:67-76`). They never reach the list, and nothing says why. That is the maintainer's "all I see is one I copied".
  - Nothing joins by itself except through the project's default pool, which takes every session no row names (`pools.rs:6-9`, `default_pool_id` at `pools.rs:113`).
- **What people run.** The vendors' own multi-agent docs and well-known public workflows share one shape:
  - a lead or orchestrator that splits the work;
  - 3–5 workers, each isolated in its own worktree or with its own files;
  - an independent reviewer with fresh context, often read-only;
  - a shared task list with claim, blocked and done states, plus direct messages;
  - a person who approves (§1).

  Teams cross tools: a Claude session writes, and a Codex session or a second Claude reviews.
- **What the tools allow.** Only sessions AI Juggler starts can carry its agent tools with an identity:
  - Claude Code and Codex get them by launch options (VP lab);
  - OpenCode, Grok Build and Kimi Code get only the command line in their shell (`crates/juggler-app/src/features/agent_bridge/mod.rs:308-336`, ADR-0020 §5).

  Sessions other apps run cannot be given tools per session:
  - the Claude desktop app's Code tab takes no launch flags (VP);
  - the ChatGPT app's Codex reads the shared `~/.codex/config.toml` (VP).

  They can only be watched, or continued in AI Juggler (§2).
- **The model we recommend** (§4).
  - Pools may be in several, by default.
  - A **scope** of every project, or a chosen set of one or more projects.
  - **Members** are rules that match sessions: a session, a profile, an account, an app, a Mac, or everyone in scope. Future sessions join by themselves.
  - Roles: Lead, Member, Reviewer and Observer, under D-28's levels, with an optional level per member.
  - **Communication** is set per role pair: allowed, ask first or blocked. One pair can be changed by hand.
  - A **sealed** pool is exclusive and closed.
  - Today's pools move over with their ids, so messages, tasks, rules, audit and layout stay attached.

---

## 1. How people run several agents today (use-case research)

### 1.1 Sources

| # | Claim | Source | Label |
|---|---|---|---|
| S1 | Agent teams: a lead plus teammates, each "a full, independent Claude Code session". They share a task list (pending, in progress, completed, with dependencies; a blocked task isn't self-claimed; claiming uses file locks) and have a mailbox per agent. Experimental and off by default. One team per session, no nesting, and the lead is fixed. "Start with 3-5 teammates". "Two teammates editing the same file leads to overwrites", so each owns different files | https://code.claude.com/docs/en/agent-teams | VP |
| S2 | Agent-team use cases named by the vendor: research and review, new modules (each teammate owns a piece), debugging with competing hypotheses, and cross-layer work (frontend, backend, tests). The worked examples are a three-lens PR review (security, performance, tests) and five adversarial investigators | same | VP |
| S3 | A teammate's message "can't approve a permission prompt or supply consent on your behalf". Teammate permission prompts appear in the lead session | same | VP |
| S4 | Cross-session messaging between any of the user's own Claude Code sessions on one Mac: `ListAgents` and `SendMessage` over a per-session Unix socket, "never through Anthropic servers". On by default from v2.1.224. Incoming messages can't approve anything or change configuration. Inbound is `crossSessionInbound` = `accept` / `hold` / `refuse`, set by settings or the `--settings` flag. A session in the VS Code extension or the Desktop app "can't show the dialog" for held messages | https://code.claude.com/docs/en/cross-session-messaging | VP |
| S5 | "Run agents in parallel" lists subagents, agent view (research preview), agent teams (experimental), Projects (public beta) and dynamic workflows. "To involve a different tool, expose it to Claude as an MCP server" | https://code.claude.com/docs/en/agents | VP (read by this task's web pass) |
| S6 | Worktrees: `claude --worktree <name>` gives each parallel session its own checkout; a subagent can take `isolation: worktree` | https://code.claude.com/docs/en/worktrees | VP (web pass) |
| S7 | Best practices: "A second session is also the easiest independent reviewer you can get." One writes and another reviews without having watched it being written. A read-only reviewer subagent is "enforced by Claude Code rather than requested in a prompt". Over-review warning: a reviewer asked to find problems "will always find something" | https://code.claude.com/docs/en/best-practices | VP (web pass) |
| S8 | Anthropic's research system is orchestrator-workers: a lead spawns 3–5 parallel subagents. Without detailed task descriptions, agents "duplicate work, leave gaps". Workers write results to storage and pass references back. Multi-agent runs use about 15× the tokens of chat | https://www.anthropic.com/engineering/multi-agent-research-system (2025-06-13) | VP (web pass) |
| S9 | Patterns: prompt chaining, routing, parallelization (sectioning and voting), orchestrator-workers, evaluator-optimizer. Add complexity "only when it demonstrably improves outcomes" | https://www.anthropic.com/engineering/building-effective-agents (2024-12-19) | VP (web pass) |
| S10 | Codex subagents: TOML agents in `~/.codex/agents/` or `.codex/agents/`; the built-in agents are `default`, `worker` and `explorer`. Children inherit sandbox and approvals and can't override MCP settings. Orchestration goes through the parent; peer messaging isn't described | https://learn.chatgpt.com/docs/agent-configuration/subagents.md | VP (web pass); NE for peer messaging |
| S11 | Codex in the ChatGPT desktop app can give each chat a worktree in `$CODEX_HOME/worktrees`. Codex cloud runs parallel tasks per environment | https://learn.chatgpt.com/docs/environments/git-worktrees.md, https://learn.chatgpt.com/docs/cloud.md | VP (web pass) |
| S12 | A public parallel-agent practice: Claude Code and Codex CLI run in separate terminals on fresh checkouts, with riskier work sent to Codex Cloud. Review is the bottleneck ("I can only focus on reviewing and landing one significant change at a time"). Research, small maintenance and well-specified tasks parallelize well | https://simonwillison.net/2025/Oct/5/parallel-coding-agents/ (2025-10-05) | VS |
| S13 | GitHub Actions runs Claude Code as an `@claude` responder or as an automation with a prompt. CI is a separate agent runtime | https://code.claude.com/docs/en/github-actions | VP (web pass) |

### 1.2 What is common (I, from S1–S13)

1. **Roles:**
   - a lead or orchestrator plans and splits;
   - implementers each own a slice;
   - an independent reviewer works from fresh context, often read-only;
   - testers or verifiers;
   - scouts research.
2. **Isolation:** each writer gets its own worktree or checkout, or its own files.
3. **Coordination:** a shared task list (claim, blocked, done) and direct messages, with results passed as files or references.
4. **People stay in the loop:** an agent's message is never consent (S3, S4). The person approves merges and risky steps, and review is the bottleneck (S12).
5. **Size and cost:** 3–5 workers. Cost grows with every agent (S1, S8).
6. **Mixed tools are normal** (S5, S12): Claude and Codex side by side. No vendor offers coordination *across* tools. That is AI Juggler's gap to fill (already found in `pools-messaging-tasks.md` §0).

### 1.3 The use cases (the stories the templates come from)

Each lists who, the setup, the pool (members, scope, roles, communication), the value, and the capabilities needed. "AJ-started" means a session AI Juggler started with its agent tools.

1. **Feature team on one project.**
   - Who: a developer with one Max account, building a feature across API, UI and tests.
   - Setup: three Claude Code sessions in three worktrees, plus one lead session.
   - Pool: scope is the one project. Members are the four sessions: one Lead and three Members. Communication: Lead to anyone, and Members to the Lead, allowed. Member to Member is "ask first", which keeps cross-talk visible.
   - Value: the agent-teams pattern (S1) without its limits. It survives `/resume`, each worker keeps its own worktree (agent teams don't isolate), tasks persist, and the person sees the canvas.
   - Needs: AJ-started sessions (tools); the tasks (E19); messages (E18).
2. **Agents across repositories.**
   - Who: someone changing an API in `backend` and its client in `web`, two repos.
   - Pool: scope is the two projects. Members are the profile "Claude Code · Work" (every session of it in those two projects joins by itself) and one Codex session. Roles: the backend session leads. Communication: all allowed.
   - Value: the cross-session messaging case "Coordinate parallel worktrees" (S4), but across repos and across tools. The Codex session hears that the schema changed.
   - Needs: several-project scope; profile members; Codex tools (VP lab).
3. **A reviewer for everything.**
   - Who: someone who wants one independent reviewer (S7) on all their work, perhaps on another tool (Codex reviewing Claude's work, S12).
   - Pool: scope is every project. Members are one Codex session as Reviewer, plus "Everyone in scope" as Members. Communication: Members to the Reviewer allowed (review requests), the Reviewer to anyone allowed, Member to Member blocked in this pool (their own project pools carry that).
   - Value: one reviewer with fresh context. It can't edit the others' tasks, only reply and move a task in review to approved or sent back. The person still approves (D-44).
   - Needs: a session in several pools (the reviewer pool and its project pool); the Reviewer role.
4. **Rules by account.**
   - Who: someone with a Work account and a Personal account who wants Personal sessions to stay read-only toward Work.
   - Pool: scope is every project. Members: account "Personal" (all its profiles, any app) at access level Read. Communication with any other pool: blocked.
   - Value: one rule covers every present and future Personal session. This is the option D-33 left to "their own pools", now as one pool instead of many.
   - Needs: account members (worked out from each profile's sign-in, §5 of the relation map); precedence when the same session is also in a project pool.
5. **A build Mac through Fleet.**
   - Who: someone with a MacBook and a Mac mini, where the mini runs long builds and test agents.
   - Pool: scope is one project. Members: the Mac "Mac mini" (every session on it in scope) as Members, and the MacBook sessions with one Lead. Spans Macs: on (D-32).
   - Value: the mini's test agent claims "run the full suite" tasks and reports back. The MacBook lead sees it on the canvas.
   - Needs: Mac members; AJ-E1705 (pool content over juggler-link, still TODO); Fleet pairing (E7, done except AJ-E710).
6. **A sealed client project.**
   - Who: a contractor whose client's code must not leak into any other work.
   - Pool: scope is the client project. Members: everyone in scope, plus the profile "Claude Code · Client".
   - Sealed means:
     - its sessions are in no other pool;
     - no links to other pools;
     - its docs and messages never leave it;
     - Claude's own cross-session messages into its Claude sessions are refused (`crossSessionInbound: refuse` through the launch's `--settings`, S4);
     - it never spans Macs unless switched on.
   - Value: one switch gives a provable boundary, shown on the canvas.
   - Needs: exclusive membership; the launch settings overlay for Claude (exists for hooks, ADR-0020 §5).
7. **Scouts (research and competing hypotheses).**
   - Who: someone hunting a bug with no clear cause (S2's "competing hypotheses").
   - Pool: scope is one project. Members are 3–5 short-lived sessions, which can be on different apps for different views. Roles: one Lead, the rest Members. Communication: all allowed, so they can challenge each other. Access: Work together, so they share docs.
   - Value: a debate in the open, with findings as shared docs.
   - Needs: messages and shared docs (E18).
8. **Writer and reviewer pair across tools.**
   - Who: someone who writes with Claude and has Codex check (S7, S12).
   - Pool: scope is one project. Members are two sessions, Lead and Reviewer.
   - Value: the smallest useful pool, and the first template most people need.
   - Needs: both tools' agent tools (VP lab).

The automatic **project pool** (today's default pool) stays as the "everyone in this project, watched together" case. It is the canvas of a project's sessions, and the place their tasks live.

---

## 2. What each tool can do in a pool (capability evidence)

### 2.1 Facts per tool

| # | Claim | Source | Label |
|---|---|---|---|
| T1 | AI Juggler gives its agent tools only to launches it makes. The adapter's `agent_bridge` returns launch options and files; the default is `None` (environment only) | `crates/juggler-adapter/src/adapter.rs:758-776`; `crates/juggler-app/src/features/agent_bridge/mod.rs:308-336` | VP (local) |
| T2 | Only Claude Code and Codex (and the test fake) declare `AGENT_BRIDGE` | `crates/juggler-adapters/src/claude_code.rs:141,176-182`; `codex.rs:472,509`; `fake.rs:76` | VP (local) |
| T3 | Identity is a per-launch key in the environment (`JUGGLER_AGENT_TOKEN`), never argv or a file. The request has no field naming a pool or session | ADR-0020 §4, §6 (`docs/adr/0020-agent-bridge.md`) | VP (local) |
| T4 | Claude Code: `--mcp-config` from AI Juggler's folder loads with no prompt, the tools are pre-allowed, and the hooks run alongside the account's own | `pools-messaging-tasks.md`, Lab 2026-10-26 (answers 2 and 4) | VP (lab) |
| T5 | Codex: `-c mcp_servers.juggler.*` with per-tool `approve`. It called `task_claim`, `task_create` and `task_assign` unprompted | same (answer 3) | VP (lab) |
| T6 | OpenCode / Grok / Kimi sessions AI Juggler starts get the key and socket in their environment, so the `juggler-agent` command line works from their shell. No MCP is injected | ADR-0020 §5 ("Tools without MCP by launch option: the environment only"); `agent_bridge/mod.rs:329-336` | VP (local) |
| T7 | OpenCode reads inline config from `OPENCODE_CONFIG_CONTENT`, which is merged above project config. MCP is the `mcp` key, `type: local` | `docs/research/opencode.md:33,85`; https://opencode.ai/docs/config/, https://opencode.ai/docs/mcp-servers/ | VP (route exists); **I** that an MCP server passed this way loads with no prompt |
| T8 | Grok Build reads an inline `GROK_CONFIG` overlay. MCP is `[mcp_servers.<name>]`. It also loads `~/.claude.json` and `.mcp.json` | `docs/research/grok.md:47,50,132`; https://docs.x.ai/build/features/mcp-servers | VP (route exists); **I** for a per-launch MCP |
| T9 | Kimi Code: MCP from `$KIMI_CODE_HOME/mcp.json` and the project's `.kimi-code/mcp.json`. No per-launch flag or inline config was found | `docs/research/kimi.md:142`; https://moonshotai.github.io/kimi-code/en/customization/mcp.html | VP (files); **NE** (per-launch route, §8) |
| T10 | Claude desktop app, Code tab: reads the same `~/.claude.json`, `.mcp.json`, hooks and `settings.json` as the CLI, and also loads `claude_desktop_config.json` servers. Per-session flags have no equivalent (`--allowedTools`: "No per-session equivalent") | https://code.claude.com/docs/en/desktop ("Shared configuration", "MCP servers from the Claude Desktop chat app", the CLI-flag table) | VP |
| T11 | Claude desktop app, chat: local MCP servers through `claude_desktop_config.json` and `.mcpb` desktop extensions | https://modelcontextprotocol.io/docs/develop/connect-local-servers; https://support.claude.com/en/articles/10949351-getting-started-with-local-mcp-servers-on-claude-desktop | VP (web pass) |
| T12 | Codex MCP config: "The ChatGPT desktop app, Codex CLI, and IDE extension share this configuration." "ChatGPT web doesn't read local Codex configuration files" | https://learn.chatgpt.com/docs/extend/mcp?surface=cli | VP |
| T13 | `codex mcp-server` has been removed; the app server replaces it (experimental) | https://learn.chatgpt.com/docs/mcp-server.md; https://learn.chatgpt.com/docs/app-server.md | VP (web pass). The removal date is unverified |
| T14 | Claude Code plugins can bundle MCP servers, skills and hooks. A user-scope plugin is available in the terminal, the desktop app's local sessions and VS Code | https://code.claude.com/docs/en/plugins | VP (web pass). The desktop page lists its own plugin manager and doesn't restate the sharing |
| T15 | User-scope MCP servers and plugins load in every Claude Code session, AI Juggler's or not, including `-p` (except `--bare` and `--strict-mcp-config`) | https://code.claude.com/docs/en/mcp; https://code.claude.com/docs/en/headless | VP (web pass) |
| T16 | AI Juggler's plugin (AJ-E1905), outside AI Juggler's launches, answers each tool call with "This session wasn't started by AI Juggler…". It reads no key | `plugins/claude-code/ai-juggler/bin/juggler-mcp:1-30`; D-43 point 6 (`docs/review/decision-log.md`) | VP (local) |
| T17 | Claude's own cross-session messages reach any of the user's Claude Code sessions on the Mac (same OS user), including the desktop app's. Inbound can be set per session with `--settings` | S4 | VP. Whether sessions under different `CLAUDE_CONFIG_DIR`s list each other is **NE** (`pools-messaging-tasks.md` §1.3) |
| T18 | Elicitation (asking the user through an MCP server) is cancelled under `-p` in Claude Code and auto-rejected in Codex unless enabled. The 2026-07-28 spec replaces server-initiated elicitation with multi round-trip requests | https://code.claude.com/docs/en/mcp; https://learn.chatgpt.com/docs/config-file/config-reference.md; https://modelcontextprotocol.io/specification/2026-07-28/changelog | VP (web pass) |
| T19 | MCP has no standard way for a server to push a message into the model's context. Claude Code channels are a research preview, plugin allowlisted | https://code.claude.com/docs/en/channels | VP (web pass) |

### 2.2 Capability matrix

The five columns:
- **See** members: the roster and cards.
- **Message**: send through the pool.
- **Tasks**: create, claim and update.
- **Be asked**: receive a message or a task and act on it.
- **Bound**: whether the pool's level and rules apply to what it does.

| Tool · runtime | See | Message | Tasks | Be asked | Bound by rules | Evidence |
|---|---|---|---|---|---|---|
| Claude Code · started by AI Juggler (terminal or driven) | Yes | Yes | Yes | Yes (inbox; delivery at Full or by a rule, D-29) | Yes | T1–T4 |
| Codex · started by AI Juggler | Yes | Yes | Yes | Yes | Yes | T1–T3, T5 |
| OpenCode · started by AI Juggler | Through its shell (command line) | Through its shell | Through its shell | Inbox, read by command | Yes (every call goes through `allowed`) | T6; MCP route T7 is **I** until a lab |
| Grok Build · started by AI Juggler | Through its shell | Through its shell | Through its shell | Inbox, read by command | Yes | T6; MCP route T8 is **I** |
| Kimi Code · started by AI Juggler | Through its shell | Through its shell | Through its shell | Inbox, read by command | Yes | T6, T9 |
| Claude Code · CLI in the user's terminal | Watched only | No | No (the user can link a task) | No (continue it in AI Juggler) | No | T3, T16. Claude's own messaging still reaches it (T17), outside AI Juggler's rules |
| Claude Code · desktop app's Code tab | Watched only | No | No | No | No | T10 (no launch flags); user-scope config would add tools to *every* session with no identity (T15) |
| Claude desktop app · chat | Watched only (if seen at all) | No | No | No | No | T11. A chat has no project folder |
| Codex · ChatGPT desktop app or IDE extension | Watched only | No | No | No | No | T12 (shared `config.toml`, no per-session option) |
| Codex · CLI in the user's terminal | Watched only | No | No | No | No | T3; the same shared config |
| Any tool · on a paired Mac (Fleet) | Same as on that Mac, once AJ-E1705 carries pool content | Same | Same | Same | Yes, with D-32 on | D-32; E17 AJ-E1705 TODO |

"Watched only" means the session can be a member and shows on the canvas and to other agents' rosters, with its state from its own files (the conversations index), but it has no tools and no rules apply. That is D-30's "pool members but can't call the tools until continued in AI Juggler", made visible.

### 2.3 What this means for the user (I)

- The full pool experience needs the session to be started (or continued) by AI Juggler. The add list must say so for every session it shows greyed out, and offer **Continue in AI Juggler** (a resume with the tools, a new launch; ADR-0020 §4).
- A user-scope bridge (plugin or `~/.claude.json`) would put the tools in every Claude session, including the desktop app's. But a session not started by AI Juggler has no key, so the tools only refuse (T16). Giving such a session an identity is the open D-43 point 6 (b). A later spike could:
  - match the bridge's peer process to a known session by process lineage, which reads no other process's environment (D-37);
  - then have the user approve the join once in AI Juggler.

  This is **I**, needs an ADR, and is not part of the first build.
- Claude's own peer messaging (T17) works between Claude sessions whatever a pool says. AI Juggler can close it only for sessions it launches, through the launch's `--settings` (`crossSessionInbound`). It can do this per receiving session, not per sender. Per-pair rules therefore govern AI Juggler's channel exactly. Claude's channel is governed per receiver: `refuse` for sealed pools, `hold` where a pair is blocked or ask first. In the desktop app, held messages expire because it "can't show the dialog" (S4).

---

## 3. What exists today (the starting point)

| Piece | Where | State |
|---|---|---|
| Pools: kinds `default` / `user`, at most one project, `spans_macs`, level, hop limit, stop-all | `crates/juggler-core/src/storage/pools.rs:46-69` | Built (v33) |
| Members: `work:` / `session:` refs; roles lead / member / observer; one active row per member across all pools | `pools.rs:70-79,156-242` | Built |
| A session with no row is in its project's default pool | `pools.rs:6-9,109-118`; `Memberships::place` (`pools.rs:320`) | Built |
| Access levels Read / Work together / Full / Custom and 13 actions; one decision point `allowed()` | `crates/juggler-core/src/agent_access.rs:25-75`; `crates/juggler-core/src/storage/agent_access.rs:288` | Built (D-28) |
| Messages, shared docs, pool links (cross-pool messages only through a user link), held asks, pauses | `crates/juggler-core/src/storage/messages.rs:31-150` | Built (E18) |
| Tasks keyed by `pool_id` | `crates/juggler-core/src/storage/tasks.rs:37-100` | Built (E19) |
| Canvas layout keyed by `(pool_id, member_ref)` | `crates/juggler-core/src/storage/pool_canvas.rs:27-35` | Built (v38) |
| The add list: AI Juggler's live sessions only, those `cannotJoin` lets through | `ui/src/pages/PoolPage.tsx:118-121`; `ui/src/pages/pool/actions.ts:24-28`; `ui/src/pages/pool/Palette.tsx:63-70` | Built; the gap the maintainer hit |
| New pool: makes "New pool" with no project and opens its empty canvas | `ui/src/screens/pools/PoolsScreen.tsx:117-133` | Built; no wizard, no template |
| The spike AJ-E1701 | `docs/tasks/E17-agent-pools.md:41` | TODO (never done) |
| Pools across Macs AJ-E1705 | `docs/tasks/E17-agent-pools.md:120` | TODO |

---

## 4. The model (recommendation, with rejected alternatives)

### 4.1 Scope

- **Choices:**
  - **Every project**, which includes sessions with no project;
  - **these projects**, one or more.
- **What scope governs:**
  1. Who can join. A broad member (profile, account, app, Mac, everyone) brings in only its sessions whose project is in scope.
  2. What agents see. The roster, cards, tasks and docs are the pool's alone. In a pool of several projects, each card and task names its project.
- A session added by name from outside the scope is refused, with the reason "works in *web*, not in this pool's projects" and a one-click "Add *web* to the scope".
- **Rejected:**
  - Scope as its own member type (it mixes "where" with "who").
  - No scope at all (every pool would leak across projects by default).
  - Only one project (the maintainer asked for several).

### 4.2 Member types and how a session is matched

| Member | Matches | Joins by itself? | Known how | Notes |
|---|---|---|---|---|
| Session | one Work item (so a switch keeps it, as today) | No | stored | today's member |
| Profile (an app on an account: a Claude Code folder, a `CODEX_HOME`, a Claude app profile) | every session of that profile in scope | Yes | stored (`instances.launch_recipe_id` → `launch_recipes.account_id`; the conversation's folder) | the maintainer's "whole app / account" |
| Account (a subscription) | every session of every profile signed in to it | Yes | worked out from each profile's sign-in, not stored (relation map §2) | changes if a profile signs in elsewhere; the pool says so |
| App (a tool: every profile of Codex, say) | every session of that tool | Yes | stored (`accounts.provider_id`) | cheap; useful for "every Codex session reviews" |
| Mac | every session on that Mac | Yes | stored (`instances.machine`) | needs a paired Mac and the pool's "spans Macs" on (D-32) |
| Everyone in scope | every session in scope | Yes | stored and inferred project | today's default pool, as a member rule |

- **Roles and rules per type.** A broad member carries one role and an optional level for every session it brings in. One session named on its own can override that.
- **Precedence inside a pool:** the most specific wins. Session > Profile > Account > App > Mac > Everyone.
- **Across pools.** A session can be in several pools (§4.3). Each pool's level, roles and communication apply to what the session does *in that pool*: every message, task and doc belongs to one pool.
- **Sealed pools.** A sealed pool's sessions are in no other pool. A broad member elsewhere skips them, and the add list shows "in *Client* (sealed)".
- **Rejected:**
  - Members as a stored list only, with nothing automatic (that is 0.3's problem).
  - "Least privilege across all pools" for every action. It looks safe, but a reviewer pool at Read would then make its reviewer read-only in its own project pool.

### 4.3 One pool per session, or several

- **Recommendation: several.** Use cases 3 and 4 need it.
- **What changes:**
  - Each bridge call that writes (message, task, doc, status) acts in one pool. With one pool there is nothing to choose.
  - With several, the agent passes the pool's id from its own `whoami`. The app checks that the session is a member, so this is not a field that can name someone else's pool.
  - ADR-0020 §6 says no field names a pool, so it needs an amendment for this.
  - Reads (`pool_roster`, `tasks_list`) default to all the session's pools, grouped by pool.
- **Rejected:**
  - One pool per session decided by precedence. A reviewer could not review everything without leaving its project pool.

### 4.4 Roles and access

- **Roles:** Lead, Member, Observer (as today, D-43 point 3), plus **Reviewer**:
  - reads every task, doc and member card in the pool;
  - messages any member;
  - takes tasks marked as reviews, and moves a task in review to approved or sent back as its own action. That action asks the user at the pool's level, and the user's final Approve (D-44) stays.
  - It doesn't claim build tasks.
- **Access:** D-28's levels stay per pool. A member may carry its own level (Read, Work together, Full, Custom) that replaces the pool's for that member. Hard lines don't move: only the user changes pools; agents never start sessions or pick accounts (D-31).
- **Rejected:**
  - New permission levels (D-28's suffice).
  - Free-form custom roles in the first build. They come back only if templates can't express a real case.

### 4.5 Communication

- **States:**
  - **Allowed**: delivered to the inbox, and woken at Full or by a rule, D-29.
  - **Ask first**: held in Needs you (E18's held message).
  - **Blocked**: refused, and the agent is told it's blocked by the pool.
- **Set by role pair** (Lead→Member, Member→Member, Member→Reviewer…), with any one pair overridable on the canvas (click the edge). Templates set the defaults.
- **Cross-pool:** still only through a pool link the user makes (`pool_links`, E18).
- **Sealed:**
  - membership is exclusive;
  - no pool links;
  - no docs or messages out;
  - for Claude sessions AI Juggler launches, `crossSessionInbound: refuse` through the launch's `--settings` (T17);
  - it never spans Macs unless switched on.

  The canvas shows a seal on the pool's header, and the add list explains it.
- **Claude's own channel** (T17), per receiving Claude session AI Juggler launches:
  - `refuse` in a sealed pool;
  - `hold` when any pair into it is Blocked or Ask first;
  - otherwise untouched.

  This is **I** until a lab confirms that `--settings` sets it for one session.
- **Rejected:**
  - Per-pair settings only (too many edges for 6+ members).
  - Per-pool on or off only (can't express "members talk only through the lead").

### 4.6 Sessions other apps run

- **Recommendation:** they can be members as **Watched**. They show on the canvas, in others' rosters (marked "can't reply") and in counts. They have no tools and no rules. One click on **Continue in AI Juggler** gives them the tools from then on.
- Messages to a Watched member are held with the reason "it can't read messages until continued in AI Juggler".
- **Rejected:**
  - Hiding them (today's confusion).
  - Adding the tools through user-scope config (every session would get tools with no identity, T15/T16, and it edits the user's setup, D-30).
- The join request (D-43 point 6 (b)) stays a spike (§6).

### 4.7 Moving today's pools (nothing lost)

| Today | Becomes | How |
|---|---|---|
| A project's default pool `pool-default:<p>` | a **project pool**: scope {p}, member "Everyone in scope (skip sessions another pool holds)" as Member; kept, never deleted or renamed | same id |
| `pool-default:none` | the no-project pool: scope "no project", Everyone | same id |
| A user pool with a project | scope {project} | same id |
| A user pool with none | scope "Every project" (today `cannotJoin` lets any session into it, `actions.ts:24-27`) | same id |
| `pool_members` rows (`work:` / `session:` + role) | Session members with the same role and `joined_at` | same rows; the global unique index becomes unique per `(pool_id, member_ref)` |
| Levels, rules, approvals, audit, messages, docs, links, pauses, tasks, layout | unchanged (all keyed by `pool_id`; layout by `(pool_id, member_ref)`) | none |

- The "skip sessions another pool holds" switch on the migrated project pools keeps 0.3's behaviour: a session placed in a user pool isn't also in its project pool. New project pools default to it too.
- Turning it off makes the project pool a watch-everything view that overlaps the others.
- Schema: additive (v40+): `pool_scope(pool_id, project)`, `pools.scope_all`, `pools.sealed`, `pool_members.level` (nullable), `pool_pairs(pool_id, from, to, rule)`, plus new `member_ref` prefixes `profile:`, `account:`, `app:`, `mac:`, `all`.
- Tasks in pools of several projects get a nullable `project` column. Today's tasks take their pool's project.

### 4.8 Limits and honest gaps

- Watched members can't act (§2.2). The fix is Continue in AI Juggler, which starts a new process: the conversation carries on, but it is not the same window.
- OpenCode, Grok and Kimi act through their shell until a lab proves a per-launch MCP route (T7, T8). Kimi has none found (T9).
- Account membership depends on reading each profile's sign-in. A profile signed out shows as "account unknown" and its sessions drop out of account members until read again.
- Mac members need AJ-E1705.
- Claude's peer channel can be closed only per receiving session, and only for AI Juggler's launches (§4.5).
- **Performance budgets** for the build:
  - a pool of 50 members opens at once from the cached roster;
  - membership resolves from in-memory maps (profile → pools, account → pools, app → pools, Mac → pools, project → pools), rebuilt on change events, never by re-reading sessions or transcripts;
  - one session's pools take O(member types) lookups.

---

## 5. The creation flow (summary; wireframes in the proposal)

- **New pool** opens the wizard. "Set up by hand" opens the empty canvas, as today.
- **Wizard steps:**
  1. Template: 8 templates from §1.3, plus Blank.
  2. Where and who: scope picker (projects, active first, with counts), then members by type (Open now, Recent, then All, with search past 8 items; every candidate listed, greyed with its reason and what would let it in).
  3. Roles and talk, with the template's defaults shown as a small role-pair grid.
  4. Review: a plain sentence per rule, and what each tool can do in this pool.
- **Manual setup on the canvas:**
  - The empty state teaches in three steps: set the scope, add members, pick a lead.
  - The palette lists every candidate by type, with reasons.
  - Edges carry communication; the inspector shows scope, members by type and sealing.

## 6. Spikes the build needs (each a lab with spare accounts, AJ-E1701's live part)

1. Two pools per session: the bridge's pool argument end to end.
2. `--settings` with `crossSessionInbound` for one Claude launch: `refuse` and `hold`.
3. `OPENCODE_CONFIG_CONTENT` and `GROK_CONFIG` carrying `juggler-agent` as an MCP server with no prompt.
4. (Later, ADR first) a join request from a session not started by AI Juggler, with identity by process lineage and the user's one-time yes.

## 7. Rejected wholesale

- **Rebuilding agent teams.** Claude's agent teams and cross-session messaging already serve Claude-to-Claude work (S1, S4). AI Juggler adds what they lack: mixed tools, durable tasks, rules the user sets, a canvas, Macs.
- **Pushing into sessions over MCP.** There is no standard push (T19). Asks belong in AI Juggler's Needs you, not in elicitation (T18).

## 8. Search log (negative evidence)

- Kimi per-launch MCP (T9). Checked `docs/research/kimi.md` (its 22 downloaded KD pages, including config-files, mcp, plugins, hooks) and https://moonshotai.github.io/kimi-code/en/customization/mcp.html, on 2026-10-30. No `--mcp-config` flag or inline config variable was found.
- Codex peer messaging (S10). The Codex subagents page and the config reference were read 2026-10-30 (web pass). There is no direct agent-to-agent messaging between threads, other than through the parent.
- Sessions under different `CLAUDE_CONFIG_DIR`s listing each other (T17). The cross-session messaging page, read 2026-10-30, names only "the same machine" and "your operating-system user". Nothing was found about config folders.

## 9. Not verified

- The exact version and date of `codex mcp-server`'s removal.
- OpenAI's post on the Codex app merging into the ChatGPT app (it returned 403).
- Grok and Kimi subagents and hooks beyond the repo's research docs.
- Elicitation in OpenCode, Grok and Kimi.
- Whether the desktop app's Code tab loads user-scope *plugins*. The plugins page says yes; the desktop page doesn't restate it.

---

## 10. Fit: how pools serve the goal and work with everything else

The maintainer, 2026-10-30, asked that every feature be reasoned against the whole: "how does this fit our overall goal, how does it fit all other capabilities we have, what are dependencies, how does it work."

### 10.1 The user value and the goal it serves

AI Juggler's goal has four parts:
- see what's open and where;
- juggle many subscriptions;
- switch between them without losing work;
- keep agents working.

Pools serve each one:

| Goal | What pools add |
|---|---|
| See what's open, where | A pool's canvas shows a *team*: who is in it, on which app and account, what each is on, and what waits on you. A pool can now span projects and apps, so one canvas can show a reviewer, its writers and a session in the Claude app (Watched). |
| Juggle many subscriptions | Account and App members let you set rules once per subscription. "Personal is read-only toward Work" stays true for every new session (use case 4). Pools never choose an account (D-31). |
| Switch | Membership hangs off the Work item, so a switch keeps the pool, its tasks and its messages (§10.2). The switch check also says what the destination can do in the pool. |
| Keep agents working | Agents hand work over through tasks and messages, wake one another only where you allowed it (D-29), and stop at the hop limit. Your rules cut the asks down to the few that matter (D-28). |

**Words.** The entity audit (`docs/research/entity-model-audit.md`, its decision 7) proposes "App" for an app in one of its profiles and "Tool" for the family. If that is ruled, this model's **Profile** member is called **App** and its **App** member is called **Tool**. The meaning doesn't change.

### 10.2 How it works with every related capability (both ways)

| Capability | What pools use from it | What it gets from pools |
|---|---|---|
| **Switching** (ADR-0011, D-03, D-51) | The Work item is the member anchor (`pools.rs:9-12`), so A→B→Codex→A stays one member. Tasks link to the Work item too (`task_sessions.work_id`, `crates/juggler-core/src/storage/tasks.rs:75`). | The switch check gains one line per pool. It warns when the destination can only act through its shell, can only be watched, or is out of scope, or when a sealed pool would drop it (detail below this table). The new launch gets a new key (ADR-0020 §4), and the pool notices nothing else. |
| **Tasks** (E19, D-43, D-44) | Tasks stay keyed by pool, with the same states, claim rule and four user actions. | The Reviewer role works tasks in review. In a pool of several projects, each task carries its project. A session's tasks are grouped by pool on its Tasks tab. |
| **Messaging** (E18, D-29) | Inboxes, held asks, delivery caps, the hop limit and pool links stay. | Talk rules per role pair decide allowed, held or refused. Claude's own channel is set per receiving session (§4.5). |
| **Usage, room and credits** (E11, E22, D-31, D-45) | The member inspector shows each member's usage. The account's room shows beside an Account member. | One line per pool: "used this week", the sum of its members' usage spans (`usage_spans`, `crates/juggler-core/src/storage/usage.rs:752`). It is a view only. Nothing in a pool ranks or picks by room (D-31); only your own launch and switch sheets suggest by room (D-45). A pool at Full that wakes sessions spends their accounts, so its header shows what it spent today. |
| **Notifications** (E25, AJ-E2501) | Asks and held messages are notices with one count (the bell, the badge, Needs you). | Each notice names its pool. Muting today is by project or kind (`notice_mutes`, AJ-E2501 req. 1), so the build adds pool as a mute origin. A pool's "what notifies you" (D-55) writes that. |
| **Processes** (E24, ADR-0010) | `juggler-agent` runs as each session's MCP child and is never cleaned up automatically (ADR-0020 §8). | A session's Processes tab names the bridge as "AI Juggler's agent tools". Watched sessions have no bridge to show. |
| **Fleet** (E7, ADR-0013, D-32) | Pairing, juggler-link, and the per-Mac View/Control permissions. | Mac members. A pool's content travels only when you switch on "spans Macs" (AJ-E1705). An offline Mac's members show offline, and messages to them queue. |
| **Local cache** (D-58, ADR-0023) | One registry of caches, invalidated by stamp, which you can clear. | Two entries: the membership maps (§4.8) and each pool's roster. Both are rebuilt on session, profile, sign-in, pool and Fleet events. Clearing the cache costs one rebuild and loses nothing. |
| **Navigation and links** (E31, D-54, D-55) | The shared list and one row (relation map §5). | Pools tabs on project, session, App, account, Mac and tool pages, with how each session got in ("through App Claude Code · Work"). The wizard's pickers are E31's pickers. |
| **The App page** (entity audit §5.1) | An App is a member type, with an id once it is stored (audit §9.3). | The App page gets a Pools tab ("this App is in 2 pools; new sessions join"). It is the natural place to "Add this App to a pool". |
| **Agent tools on or off** (D-30) | The tools can be switched off per project or profile (`crates/juggler-app/src/features/agent_bridge/settings.rs:60-96`). | A member whose tools are off shows "tools off for this App", with a link to Settings › Agent access, instead of looking broken. |

**The switch check, in detail.** For each pool the session is in, the check says one of:
- the destination can only act through its shell;
- the destination can only be watched;
- the destination is out of scope;
- a sealed pool would drop it.

### 10.3 Dependencies

- **This needs:**
  - E17's bridge, E18 and E19 (built);
  - E28's canvas and Pools screen (AJ-E2807/2808, in progress);
  - the entity audit's App and Account (decision 4). Without stored Apps, App members key on the config folder as today, and Account members stay worked out;
  - D-58's cache;
  - the labs in AJ-E3002;
  - for Mac members, AJ-E1705 and AJ-E710.
- **This is needed by:**
  - E31's Pools tabs;
  - the switch check's pool lines;
  - Needs you and notification muting by pool;
  - the Tasks screen's pool filter;
  - the guide.

### 10.4 End to end, with edge cases

**A walk through.**
1. New pool → *Writer and reviewer* → scope "AI Juggler".
2. Who: App "Claude Code · Work" as Lead, and one Codex session as Reviewer.
3. Review → Make. The canvas opens with the two nodes and one green edge.
4. The Claude session claims a task, finishes and marks it in review. The Reviewer gets the review task and sends it back with a note. The writer fixes it, and you Approve (D-44).
5. The writer runs out of room. You switch it to Codex · Personal from the switch sheet. The pool line says "stays in *Login*; Codex acts here with full tools". The member keeps its tasks and messages.
6. A Claude app session in the same project appears in the add list greyed as "watched only". You add it as Watched, then click Continue in AI Juggler. Its next launch has the tools, and the node turns solid.

**Edge cases.**

| Case | What happens |
|---|---|
| A session matched by a member in a sealed pool and by a broad member elsewhere | The sealed pool wins. The other pool shows it greyed: "in *Client* (sealed)". |
| An App signs in to another account | Account members re-resolve. The pool says "Personal no longer covers *Claude Code · Old*". |
| A sign-in can't be read | "Account unknown". The session stays in through any other member. |
| A switch to a tool that can't carry the tools, or to another app | The member stays, as shell-only or Watched. The switch check said so first. |
| A switch into an App that a sealed pool holds | The check warns that it will leave *Login* for *Client* (sealed), and you confirm or cancel. |
| The session ends | Its node fades. Its tasks follow D-44: you free a stuck task. Its membership stays with the Work item for the next switch or resume. |
| The pool is deleted | Sessions keep their other pools. Sessions with none are back in their project pool. History stays (`pools.rs:13-15`). |
| A scoped project is renamed, moved or removed | Scope holds the project id, so a rename or move follows. A removed project leaves the scope with a notice. |
| Two pools at Full both wake one session | Each delivery goes through the session's own rate limit and each thread's hop limit (D-29). Stop-all is per pool. |
| Conflicting talk rules for one pair in two pools | Each pool's rule governs messages sent in that pool. Claude's own channel takes the strictest (refuse > hold > accept) for that receiver. |
| A big pool (50+) | Broad members draw as one group node with a count. The roster and cards load from the cache. |
| A Mac goes offline | Its members show offline and messages queue. Nothing is lost (D-32, ADR-0013 offline rules). |
| The agent tools are switched off for a project | Its sessions are members, with "tools off" and the link to switch them on. |

### 10.5 What changes for existing users

- Every pool, member, role, level, rule, message, doc, task and canvas position stays (§4.7).
- A project's own pool looks the same. A session you moved into one of your pools still isn't in its project pool.
- Your pools without a project now say "Every project", which is what they already allowed.
- New:
  - a session can be in more than one pool;
  - the add list shows every session, with reasons;
  - New pool opens the wizard ("Set up by hand" is one click away);
  - Pools tabs appear on other pages.
- Agents in a single pool see no change. Agents in several pools pass the pool they mean.
