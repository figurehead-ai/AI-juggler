# E17 — Agent pools and the agent bridge

> Source: the maintainer, 2026-10-21: "Groups / Pools of different agents,
> with default group / pool always created. Many other features use this
> group / pool effect." Research: `docs/research/pools-messaging-tasks.md`
> (about 35 primary sources, accessed 2026-10-21).
>
> Claude Code sessions on one Mac can already message each other. Nothing
> joins different tools, or gives the user one place to see and control it.
> E17 builds pools and a read-only bridge; messaging (E18) and tasks (E19)
> build on both.
>
> **Law for every task here:**
> - **The user owns every pool.** Pools, roles and permissions change only
>   in the app, never through the bridge.
> - **Agent-initiated only (D-28, to be ruled).** Agents call AI Juggler's
>   tools from inside their own session. AI Juggler never types into a
>   session for them.
> - **Per-launch files in AI Juggler's own folder (ADR-0018).** The bridge
>   is added through launch flags and overlay files, never written into the
>   user's setup.
> - **No quota routing (D-31).** Nothing in a pool chooses an account by
>   remaining quota.
> - **Work and personal stay apart (D-33).** The default pool covers one
>   project and one identity.

### AJ-E1701 The spike, ADR-0019 and the policy rulings | Status: TODO | Priority P0 | Effort S | Depends: —
**Goal**: The bridge's approach is proven on real tools, and the policy is written before code relies on it.
**User story**: As the maintainer, I rule on D-28 to D-33, and I see the bridge load in Claude Code and Codex without prompts.
**Requirements**:
1. ADR-0019 "Agent bridge", plus D-28–D-33 in the decision log, as the maintainer rules them (the plan doc has the recommendations).
2. A spike on spare accounts:
   - `--mcp-config` loads in Claude Code without an approval prompt;
   - Codex's `-c mcp_servers.*` with `env_vars` passes the token;
   - a hook from a `--settings` file adds a turn note.
   The captures go to `docs/research/evidence/`.
**UI/UX reasoning**: n/a.
**Performance budget**: n/a.
**Quality gates**: the evidence standard.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1701/manifest.md`.
**Visual testing**: n/a.
**Files to touch**: `docs/adr/0019-agent-bridge.md`, `docs/review/decision-log.md`.
**Research basis**: `pools-messaging-tasks.md` §2–3.
**Handover note**: The live part of the spike needs spare accounts, which is the maintainer's step.

### AJ-E1702 Pools, the default pool, roles and permissions | Status: TODO | Priority P0 | Effort M | Depends: —
**Goal**: Sessions belong to pools that the user creates and controls.
**User story**: As a user running a planner and two builders, I put them in a pool called "Checkout redesign" and give the planner the lead.
**Requirements**:
1. Storage:
   - a pool has a name, a kind (default or user) and an optional project;
   - members hang off the Work item, so membership survives switches;
   - roles are lead, member and observer, with the user as the implicit owner;
   - the permission table is in the plan doc, with UI edits only.
2. The default pool "All sessions" always exists, can't be deleted, and covers one project and one identity (D-33).
3. UI: create, rename and delete pools (except the default); move sessions between pools; set roles; a pool filter on Home and the rail. All reuse the existing rows, headers and folds.
4. A guardrail test: pool operations never spawn, stop or type into a session.
**UI/UX reasoning**: D-25: pools read like the rail's project groups, with the same fold and mark.
**Performance budget**: the rail's budget is unchanged at 50 sessions.
**Quality gates**: migration tests (with the pre-migration copy); role and permission tests; the guardrail.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1702/manifest.md`.
**Visual testing**: pools in the rail, the pool sheet and the filter.
**Files to touch**: `crates/juggler-core/src/storage/` (pools), `commands.rs`, `WorkflowRail.tsx`, `Home.tsx`.
**Research basis**: `pools-messaging-tasks.md` §2.3.
**Handover note**: The schema bump needs a downgrade note and a `SCHEMA_RELEASES` entry.

### AJ-E1703 Context cards | Status: TODO | Priority P1 | Effort S | Depends: AJ-E1702
**Goal**: Each pool shows what every member is working on.
**User story**: As a user, I open a pool and see each agent's place, state, task and last activity at a glance.
**Requirements**:
1. Cards are built only from what AI Juggler knows: name and place, tool and state, the current task (E19), a line the agent reports itself (marked as said by the agent), last activity and recently touched files.
2. Missing data says "Unavailable"; account details are never shown to agents.
**UI/UX reasoning**: Rows like Home's, with detail behind disclosure.
**Performance budget**: built within the read model's tick.
**Quality gates**: honest-field tests (D-06).
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1703/manifest.md`.
**Visual testing**: the pool view with cards.
**Files to touch**: `commands.rs`, `RightInspector.tsx` (a pool selection).
**Research basis**: `pools-messaging-tasks.md` §2.3 (context cards).
**Handover note**: —

### AJ-E1704 The agent bridge, read-only | Status: TODO | Priority P0 | Effort M | Depends: AJ-E1701, AJ-E1702
**Goal**: Agents in any tool can ask AI Juggler who they are and who's in their pool.
**User story**: As an agent in a Codex session, I call `pool_roster` and see my teammates in Claude Code.
**Requirements**:
1. A `juggler-agent` program in the signed bundle, a new crate. It works as an MCP stdio server with `--mcp`, and as a plain command line otherwise, so tools without MCP can call it from their shell.
2. It talks to the core over a per-edition Unix socket (0600) and holds no state.
3. Identity: a random token per session epoch, passed in the environment and never in argv, and rotated on every switch or resume.
4. Injection (D-30 decides the default):
   - Claude Code gets `--mcp-config` with a per-session file in AI Juggler's overlay folder, never `--strict-mcp-config`;
   - Codex gets `-c mcp_servers.juggler.*` with `default_tools_approval_mode="writes"`.
5. Tools: `whoami`, `pool_roster`, `set_status`, plus a cards resource.
6. Tests: a fake tool acting as an MCP client through the real binary; assertions that no token is in argv and nothing is written under the account's folder; the ledger shows the helper as an MCP server, excluded from cleanup.
**UI/UX reasoning**: Invisible, apart from the opt-in question (D-30) and the status lines agents set.
**Performance budget**: a tool call answers in < 50 ms.
**Quality gates**: the tests above; the conformance kit extended with the bridge's checks.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1704/manifest.md`.
**Visual testing**: the opt-in question; status lines on cards.
**Files to touch**: `crates/juggler-agentbridge/` (new), the adapters' launch specs, `crates/juggler-app/src/`.
**Research basis**: `pools-messaging-tasks.md` §2.3 (components, how each tool gets it).
**Handover note**: The bundle must sign and notarize the new binary; check the release workflow.
