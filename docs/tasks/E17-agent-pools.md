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
> **Law for every task here** (the maintainer's rulings of 2026-10-24,
> recorded as D-28 to D-33 in the decision log):
> - **The user owns every pool.** Pools, roles, access levels and rules
>   change only in the app, never through the bridge.
> - **Access per pool (D-28).** Read, Work together (where every pool
>   starts), Full, or Custom: a level is a set of agent actions from a typed
>   catalog. Above the level is an ask in Needs you (Allow once / Always
>   allow in this pool / Deny), never a block; the second identical
>   approval offers the rule. Rules are listed and revocable, effective on
>   the next call; every agent action is audited. Hard lines no level or
>   rule lifts: agents never change pools, levels or rules, never start a
>   session or choose an account, never get credentials, and AI Juggler
>   never answers a tool's own permission prompts.
> - **Delivery into sessions (D-29).** Only at Full or by a user rule;
>   between turns, labelled, rate-limited, duplicates dropped; each pool
>   has a hop limit (default 5) that pauses and asks, and a stop-all.
> - **Per-launch files in AI Juggler's own folder (ADR-0018).** The bridge
>   is added through launch flags and overlay files, never written into the
>   user's setup.
> - **Agents never launch, no quota routing (D-31).** `prepare_launch`
>   only stages a launch for the user's click in Needs you (so Work
>   together and Full allow it without asking first); the user picks the
>   account; nothing reads quota for it.
> - **Pools across Macs (D-32).** Opt-in per pool, off by default.
> - **A project's default pool holds all its sessions (D-33)**, whatever
>   the account. Cards may show an account's label, never an email, a plan
>   or another detail.

### AJ-E1701 The spike, ADR-0020 and the policy rulings | Status: TODO | Priority P0 | Effort S | Depends: —
**Goal**: The bridge's approach is proven on real tools, and the policy is written before code relies on it.
**User story**: As the maintainer, I rule on D-28 to D-33 (ruled 2026-10-24), and I see the bridge load in Claude Code and Codex without prompts.
**Requirements**:
1. ADR-0020 "Agent bridge", plus D-28–D-33 in the decision log, as the maintainer rules them (the plan doc has the recommendations).
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
**Files to touch**: `docs/adr/0020-agent-bridge.md`, `docs/review/decision-log.md`.
**Research basis**: `pools-messaging-tasks.md` §2–3.
**Handover note**: The live part of the spike needs spare accounts, which is the maintainer's step.

### AJ-E1702 Pools, the default pool, roles and permissions | Status: DONE-EVIDENCED (IN-REVIEW until the release PR) | Priority P0 | Effort M | Depends: —
**Goal**: Sessions belong to pools that the user creates and controls.
**User story**: As a user running a planner and two builders, I put them in a pool called "Checkout redesign" and give the planner the lead.
**Requirements**:
1. Storage:
   - a pool has a name, a kind (default or user) and an optional project;
   - members hang off the Work item, so membership survives switches;
   - roles are lead, member and observer, with the user as the implicit owner;
   - permissions are D-28's access levels, rules and audit (see the law above), with UI edits only, behind one decision point (`allowed(pool, action) -> Allow | Ask | Deny`) the bridge and messaging only call; each pool also stores D-29's hop limit and stop-all and D-32's spans-Macs flag.
2. Each project has its own default pool ("<project> · all sessions"), made when the project first appears, always there, not renamable or deletable, holding every session of that project on any account (D-33); sessions with no project are in "No project · all sessions". Levels, rules, hop limits and stop-all are set per default pool like any other.
3. UI: create, rename and delete pools (except the default); move sessions between pools; set roles; a pool filter on Home and the rail; the level picker on the pool's page and Settings › Agent access. All reuse the existing rows, headers and folds.
4. A guardrail test: pool operations never spawn, stop or type into a session.
**UI/UX reasoning**: D-25: pools read like the rail's project groups, with the same fold and mark.
**Performance budget**: the rail's budget is unchanged at 50 sessions.
**Quality gates**: migration tests (with the pre-migration copy, from 0.1.0's schema 32 to 33); role, level, rule and decision-point tests; the guardrails (no path to a session; the hard lines; no quota).
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1702/manifest.md`.
**Visual testing**: pools in the rail, the pool sheet and the filter.
**Files to touch**: `crates/juggler-core/src/storage/` (pools), `commands.rs`, `WorkflowRail.tsx`, `Home.tsx`.
**Research basis**: `pools-messaging-tasks.md` §2.3.
**Handover note**: The schema bump (v33) needs a downgrade note in 0.2's release notes and a `SCHEMA_RELEASES` entry when 0.2 ships (0.1.0 = 32 is listed).

### AJ-E1703 Context cards | Status: DONE-EVIDENCED (IN-REVIEW until the release PR) | Priority P1 | Effort S | Depends: AJ-E1702
**Goal**: Each pool shows what every member is working on.
**User story**: As a user, I open a pool and see each agent's place, state, task and last activity at a glance.
**Requirements**:
1. Cards are built only from what AI Juggler knows: name and place, tool and state, the current task (E19), a line the agent reports itself (marked as said by the agent), last activity and recently touched files.
2. Missing data says "Unavailable"; of the account, a card shows only its label, never an email, a plan or another detail (D-33).
**UI/UX reasoning**: Rows like Home's, with detail behind disclosure.
**Performance budget**: built within the read model's tick.
**Quality gates**: honest-field tests (D-06).
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1703/manifest.md`.
**Visual testing**: the pool view with cards.
**Files to touch**: `features/pools/cards.rs`, the pool page (`pages/PoolPage.tsx`, a pool selection beside the inspector's pages).
**Research basis**: `pools-messaging-tasks.md` §2.3 (context cards).
**Handover note**: —

### AJ-E1704 The agent bridge, read-only | Status: TODO | Priority P0 | Effort M | Depends: AJ-E1701, AJ-E1702
**Goal**: Agents in any tool can ask AI Juggler who they are and who's in their pool.
**User story**: As an agent in a Codex session, I call `pool_roster` and see my teammates in Claude Code.
**Requirements**:
1. A `juggler-agent` program in the signed bundle, a new crate. It works as an MCP stdio server with `--mcp`, and as a plain command line otherwise, so tools without MCP can call it from their shell.
2. It talks to the core over a per-edition Unix socket (0600) and holds no state.
3. Identity: a random token per session epoch, passed in the environment and never in argv, and rotated on every switch or resume.
4. Injection (D-30: on for sessions AI Juggler starts after one question, off per project or account; AI Juggler's own tools pre-allowed):
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

### AJ-E1705 Pools across your Macs | Status: TODO | Priority P1 | Effort M | Depends: AJ-E1702, AJ-E1801, AJ-E710
**Goal**: A pool can include sessions on the user's paired Macs, when the user switches it on for that pool (D-32).
**User story**: As the user, I switch on "Include my other Macs" for my *Backend* pool, and the agent on my MacBook sees and messages the one on my Mac mini.
**Requirements**:
1. Per pool, off by default; switching it on says, in one line, that message, shared-doc and task content will travel to the user's other Macs.
2. Pool membership, statuses, messages, shared docs and tasks travel over juggler-link (mTLS, paired Macs only, no server), and only for pools switched on; switching off stops it at once.
3. Every check runs on the Mac that owns the session: `allowed(pool, action)`, the hop limit and the caps hold across Macs; a peer never decides for another Mac's sessions.
4. The Privacy page's Fleet line and the link contract tests (`crates/juggler-link/tests/proto_contract.rs`) are rewritten to say exactly when pool content travels; a test proves nothing travels for a pool switched off.
**UI/UX reasoning**: One switch on the pool's page; members on other Macs show their Mac's name.
**Performance budget**: a message reaches a peer in < 1 s on the local network.
**Quality gates**: the link tests above; the network allowlist test; the trust tests.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1705/manifest.md`, including the two-Mac check (AJ-E710, the maintainer's).
**Visual testing**: the switch and its note; a member on another Mac.
**Files to touch**: `crates/juggler-link/`, the pools module, `docs/guide/trust.md`, `crates/juggler-app/tests/trust.rs`.
**Research basis**: `pools-messaging-tasks.md` §2 (P6 fleet pools).
**Handover note**: Ruled into 0.2 by the maintainer, 2026-10-24.
