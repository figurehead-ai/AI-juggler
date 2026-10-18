# 06 — External Developer Pain Research (Second Pass)

> Parent: maintainer canonical workflow in
> [expected-worfklow.md](../../expected-worfklow.md#L1).
> Prior research cited as: Maximum-Depth Report (MDR) §01–§13 in
> [AI-Juggler-Maximum-Depth-Report-and-Proposal.md](../AI-Juggler-Maximum-Depth-Report-and-Proposal.md#L1)
> and Competitive UX Review (CUX) F1–F7 in
> [full-competitive-ux-review.md](../full-competitive-ux-review.md#L1).
> This file does not restate either report. It extends them where
> external developer evidence was under-weighted, wrong, or missed.
> Method: 20 web searches (log in §12), 2026-10-13. Every claim
> carries a repo `file:line`, a prior-report section, or an
> external URL. `[verified]` = direct external source read;
> `[inferred]` = synthesis, discount accordingly.

## 0. Second-pass verdict in eight lines

- Prior reports are strong on quota visibility and handover
  mechanics (MDR §06, §09; CUX F1.5, F3) but treat the quota
  wall as a *display* problem. External evidence shows it is a
  *timing and trust* problem: developers cannot predict resets,
  do not trust the percentage, and discover exhaustion only
  mid-task (see §2). `[verified]`
- Prior reports under-weight *silent* failures: wrong-account
  billing without any UI signal, orphan dev servers holding
  ports for days, and MCP/config fork per account. Each has
  shipped community workarounds Juggler does not yet spec
  against (see §4, §6, §8). `[verified]`
- Prior reports mark worktree collisions `[inferred]` (CUX
  pain #7). External evidence promotes this to verified,
  repeated data loss — silent overwrite with no conflict
  marker (see §7). `[verified]`
- The maintainer workflow demands switching "as many times as
  I want, back and forth" in
  [expected-worfklow.md](../../expected-worfklow.md#L7).
  Prior handover design (MDR §09) is A→B→A linear. External
  switcher tools already do pool round-robin across 3+ seats;
  Juggler's lineage model has no pool view (see §3). `[verified]`
- Gemini CLI is absent from SPEC adapters
  ([SPEC.md](../../SPEC.md#L206))
  yet external pain (429 tier mismatch, per-family quotas) is
  structurally identical to Claude/Codex. Deferring it leaves
  the "multiple tools" promise in
  [expected-worfklow.md](../../expected-worfklow.md#L3)
  unproven for a third of the named workflow. `[inferred]`

## 1. Terminology: maintainer language vs. repo domain

Maintainer language is normative per
[expected-worfklow.md](../../expected-worfklow.md#L11):
"we should call them providers, accounts, and instances of
the tool". The repo does not yet speak this language.

| Maintainer term | Repo term today | Where | Conflict |
|---|---|---|---|
| Provider | Harness / `HarnessId` | [SPEC.md](../../SPEC.md#L91) | Rename risk |
| Account | `Account` | [SPEC.md](../../SPEC.md#L93) | Aligned |
| Instance (tool instance) | `SessionRecord` + Instance UI | [SPEC.md](../../SPEC.md#L108) | Split model |
| — (no maintainer word) | `Profile` | [SPEC.md](../../SPEC.md#L101) | Extra concept |

Findings:

- **F-T1. `Provider` does not exist in the domain model.**
  `HarnessId` (`"claude-code" | "codex" | "zcode"`) in
  [SPEC.md](../../SPEC.md#L91)
  is the provider, but UI copy, onboarding, and the expected
  workflow all say "provider". New users meet two words for
  one thing. Recommendation: rename the user-facing label to
  Provider everywhere; keep `HarnessId` as the internal type
  only if a migration note records the alias. `[verified]`
- **F-T2. `Profile` is an unmaintainer concept users must
  learn.** `Profile = account + settings_overlay` in
  [SPEC.md](../../SPEC.md#L101)
  has no counterpart in the maintainer workflow, which speaks
  only of switching accounts on a project
  ([expected-worfklow.md](../../expected-worfklow.md#L5)).
  CUX F2.2 already flags Instances ≠ Sessions conflation;
  Profile adds a third noun. Recommendation: demote Profile
  to an advanced/settings concept; default UX speaks
  Provider → Account → Instance only. `[inferred]`
- **F-T3. `Instance` is split across two repo concepts.**
  SPEC §9.1 defines Instances as "a CLI running in an
  isolated terminal *or* a desktop harness window" in
  [SPEC.md](../../SPEC.md#L320),
  while `SessionRecord` in
  [SPEC.md](../../SPEC.md#L108)
  is the persisted conversation. Users think "my instance"
  = one running tool; the repo forces them to also think
  "session inside instance". Recommendation: make Instance
  the primary noun (one running tool process tree); Session
  becomes its conversation history, shown inside the
  Instance panel, never as a peer top-level object.
  `[inferred]`

## 2. Pain cluster P1 — Quota walls hit mid-task, without warning

**What prior research said.** MDR §06 grades quota numbers by
honesty tier; CUX F1.5 asks for a reset-queue. Both frame the
fix as better display plus scheduled continuation.

**What external evidence adds.** The wall is discovered
*during* work, not before it, and the in-flight work is the
casualty — a dimension prior reports mention only via repo
issues (PROPOSAL §2) without external corroboration.

- "You've probably been blindsided mid-conversation when
  Claude just stops and you realize you hit the 5-hour
  limit" — menu-bar tool built solely to prevent this
  [aiquotabar POST_PLAN](https://github.com/yagcioglutoprak/aiquotabar/blob/HEAD/docs/planning/POST_PLAN.md).
  `[verified]`
- "When Claude Code runs out mid-task, you can carry on in
  Codex" is now a documented cross-tool handoff pattern, not
  an edge case [dev.to usage guide](https://dev.to/heylittlepan/how-to-check-your-claude-code-and-codex-usage-limits-34mf).
  `[verified]`
- Max 20x users report "jumps from 21% to 100% usage on a
  single prompt" and Max 5x windows exhausted in "~1.5 hours
  with normal agentic tasks"
  [claude-code #38335](https://github.com/anthropics/claude-code/issues/38335).
  `[verified]`
- A dedicated auto-retry tool exists whose entire job is to
  wait for the printed reset ("resets 3pm") and send
  "continue" automatically
  [claude-auto-retry](https://github.com/cheapestinference/claude-auto-retry).
  `[verified]`

| Signal | Reading |
|---|---|
| Frequency | Very high — 724 issues cite the 5h limit per [PROPOSAL.md](../../PROPOSAL.md#L23); plus standalone retry/quota-bar tools |
| Severity | High — in-flight task aborts; context must be rebuilt elsewhere |
| Workaround today | Menu-bar quota widgets, auto-retry scripts, manual tool switch |
| Juggler opportunity | Pre-flight forecast + mid-task early warning, not just rings |

**Feature implication.** SPEC §6 burn-down projection
("exhausts in ~40 min") in
[SPEC.md](../../SPEC.md#L257)
must fire *inside* the running Instance as a passive alert
at configurable headroom (e.g. 20% left), with a one-click
"prepare handover" staging action. A dashboard ring the user
must go and look at does not solve a mid-task wall.
`[inferred]`

## 3. Pain cluster P2 — Reset timing is opaque and distrusted

**What prior research said.** MDR §06 notes native baselines
rose (statusline `rate_limits`, Codex `/status`). It does not
interrogate whether users *believe* the numbers.

**What external evidence adds.** Three distinct trust
failures, none in prior reports:

- **Direction confusion.** "Claude Code and Grok show how
  much you've used. Codex shows how much is left. '30%' in
  Claude Code is close to '70% left' in Codex"
  [dev.to usage guide](https://dev.to/heylittlepan/how-to-check-your-claude-code-and-codex-usage-limits-34mf).
  A unified Juggler ring that does not normalise direction
  will mislead exactly the multi-tool users it targets.
  `[verified]`
- **Contradictory windows.** "When the weekly usage limit
  reaches 0%, the 5-hour limit can still show substantial
  remaining capacity (63%) even though that capacity is
  unusable. This is misleading UX"
  [OpenAI Community](https://community.openai.com/t/codex-usage-ui-is-misleading-when-weekly-reaches-0-but-5h-still-shows-remaining-quota/1404599).
  `[verified]`
- **Reset semantics unknown.** "Claude does not reset at
  midnight. Your limits operate on a dynamic rolling
  window" and "starting a new conversation doesn't reset
  either window" are facts users must learn from third-party
  guides, not from the tool
  [specweave guide](https://github.com/anton-abyzov/specweave/blob/HEAD/docs-site/docs/guides/claude-code-usage-limit.md).
  `[verified]`

| Signal | Reading |
|---|---|
| Frequency | High — every multi-tool user crosses both conventions daily |
| Severity | Medium — wrong "plenty left" reading causes the P1 wall |
| Workaround today | Third-party guides, statusline countdown scripts |
| Juggler opportunity | Single normalised "headroom + resets-at" language |

**Feature implication.** Every quota surface must show, in
one line: headroom remaining (never used-%), the binding
window (5h vs weekly, whichever exhausts first), and an
absolute reset time with countdown. SPEC §9.2 Overview
"quota rings with reset countdowns" in
[SPEC.md](../../SPEC.md#L345)
should be amended to mandate the binding-window rule; a ring
per window invites the Codex 63%-but-unusable error.
`[inferred]`

## 4. Pain cluster P3 — Account switching is manual alias folklore

**What prior research said.** PROPOSAL §2 and MDR §06 cover
switchers (`cc-switch`, `cswap`, `aisw`) as competitors.
They under-state how *fragmented* the workaround ecosystem
is: every developer hand-rolls the same alias independently.

- "Switching is manual. You need to log out, log back in,
  or juggle `CLAUDE_CONFIG_DIR` yourself" and "settings
  don't carry over — MCP servers, plugins, permissions have
  to be set up from scratch, every time"
  [clausona](https://github.com/scalabl3/clausona).
  `[verified]`
- The canonical workaround is a shell alias pair:
  `alias claude-account1="CLAUDE_CONFIG_DIR=~/.claude-account1 claude"`
  [GitHub Gist](https://gist.github.com/sonnyit/20f7198c27be7ad92606203fdfc66019).
  `[verified]`
- Rotation tools pick "the account with most 5h/weekly
  quota headroom" automatically
  [claude-rotate](https://github.com/evrenverse/claude-rotate);
  another polls "each account's usage via the same OAuth
  endpoint Claude Code itself uses for `/usage`"
  [claude-usage-swap](https://github.com/jonazri/claude-usage-swap).
  `[verified]`
- HN validated the pain: "Show HN: Claude-account – switch
  Claude Code accounts without logging in again" (47 points,
  24 comments); typical reaction "why isn't this built in"
  [HN digest](https://github.com/xiehd77-del/agents-radar/issues/361).
  `[verified]`
- The feature request for native support asks for
  `claude auth list / switch / add` modelled on `gh auth`
  [claude-code #37554](https://github.com/anthropics/claude-code/issues/37554).
  `[verified]`

| Signal | Reading |
|---|---|
| Frequency | Very high — 6+ independent switcher tools plus gist folklore |
| Severity | Medium — daily friction; error-prone under time pressure |
| Workaround today | Aliases, snapshot-swap scripts, auto-rotators |
| Juggler opportunity | Managed Accounts with zero shell knowledge required |

**Where prior research is wrong.** MDR §06 treats
auto-rotation as a competitor behaviour to avoid (correct
per PROPOSAL guardrail 2 in
[PROPOSAL.md](../../PROPOSAL.md#L63)).
But it misses that rotation tools prove users want a *pool
view*: all Accounts of one Provider ranked by headroom, with
the switch one click away. Juggler's per-Account cards in
[SPEC.md](../../SPEC.md#L346)
have no ranked-pool ordering. **Feature implication:** the
Accounts screen and the handover target list must sort by
live headroom descending by default, so the compliant
(user-clicked) switch feels as fast as the non-compliant
auto-rotator. `[inferred]`

## 5. Pain cluster P4 — `CLAUDE_CONFIG_DIR` isolation forks everything

**What prior research said.** SPEC §3 presents
`CLAUDE_CONFIG_DIR` isolation as the verified mechanism in
[SPEC.md](../../SPEC.md#L132).
MDR §03 audits onboarding import. Neither report prices the
*ongoing* cost of isolation: everything forks per Account.

- "`CLAUDE_CONFIG_DIR` moves the entire data directory, so
  conversation history and memory fork per account"
  [sub-switcher README](https://github.com/whi02/sub-switcher/blob/HEAD/README.en.md).
  `[verified]`
- "Each account gets its own config directory, so your MCP
  servers, plugins, permissions, and settings have to be
  set up from scratch — every time"
  [clausona](https://github.com/scalabl3/clausona).
  `[verified]`
- Cross-tool MCP duplication is the same disease one level
  up: "Configure MCP once, use it in Claude, Cursor, and
  Codex" exists because "each client made its own choice"
  for config location and format
  [Medium, MCP gateway](https://medium.com/@jaegercode/configure-mcp-once-use-it-in-claude-cursor-and-codex-6685b5d28d08).
  `[verified]`
- Duplicate MCP processes launch silently when two config
  sources declare the same server
  [claude-code #66474](https://github.com/anthropics/claude-code/issues/66474).
  `[verified]`

| Signal | Reading |
|---|---|
| Frequency | High — hits every added Account, every added MCP server |
| Severity | Medium — hours of re-setup; silent duplicate processes |
| Workaround today | Hand-copied configs, `--mcp-config` overlays, symlinks |
| Juggler opportunity | Shared-settings layer across Accounts of a Provider |

**Feature implication.** CUX F1.9 ("settings/MCP/rules
portability") is scoped as a P2 checkbox sync. External
evidence says this is *onboarding-blocking*: the second
Account is unusable until MCP/plugins/permissions are
rebuilt. Promote to P1: on Account creation, offer "copy
shared settings from Account X" (MCP servers, plugins,
permissions, model defaults) as a default-on step in the
wizard, diff-previewed. Credentials never copy (SPEC §8 in
[SPEC.md](../../SPEC.md#L310)).
`[inferred]`

## 6. Pain cluster P5 — Context loss on switch; rules duplication

**What prior research said.** MDR §09 designs handover
tiers T1–T4; SPEC §7.2 flow 4 syncs rules files in
[SPEC.md](../../SPEC.md#L289).
Both assume the *conversation* is the context that matters.

**What external evidence adds.** Developers lose three
contexts on switch, and prior reports price only the first:

1. **Conversation context** (priced): "switching accounts
   means logging out, logging back in, and losing whatever
   session state you had"
   [dev.to multi-account guide](https://dev.to/sakibullah2006/running-multiple-claude-code-accounts-on-linux-opj).
   `[verified]`
2. **Rules context** (under-priced): teams maintain
   `CLAUDE.md`, `AGENTS.md`, `.cursorrules`,
   `copilot-instructions.md` in parallel; tooling now exists
   solely to keep them "single-sourced and honest" with
   `CLAUDE.md` as a one-line `@AGENTS.md` include
   [agent-standard](https://github.com/anmoln7/agent-standard-oss).
   ETH Zurich research cited there finds auto-generated
   context files "reduce agent success rates ~2% and
   increase costs 20%+"
   [quality-of-my-claude-md](https://github.com/riyazsarah/quality-of-my-claude-md).
   `[verified]`
3. **Working context** (unpriced): uncommitted diff, running
   dev server, shell env, and in-session todos do not travel
   with a transcript copy. SPEC §7.3 packs cover diffs in
   [SPEC.md](../../SPEC.md#L303)
   but no prior section inventories dev-server/env state.
   `[inferred]`

Positive signal: "Claude Code v2.1.277 reads AGENTS.md when
CLAUDE.md is missing" removes one duplication, but "leaves
the other (keys, billing, model switching) untouched"
[dev.to](https://dev.to/cogumellum/claude-code-v21277-reads-agentsmd-when-claudemd-is-missing-53ln).
SPEC already notes this in
[SPEC.md](../../SPEC.md#L289).
`[verified]`

| Signal | Reading |
|---|---|
| Frequency | High — every cross-Provider or cross-Account switch |
| Severity | High — re-explaining costs 10–30 min and loses decisions |
| Workaround today | Symlinks, `@AGENTS.md` imports, copy-paste briefs |
| Juggler opportunity | Handover carries rules + diff + env manifest, verified |

**Feature implication.** The handover wizard must show a
context checklist (conversation ✓, rules files ✓/diff,
uncommitted diff ✓, MCP/settings ✓, dev-server/env —
declared or explicitly dropped) before execute. Today SPEC
§7.2 in
[SPEC.md](../../SPEC.md#L287)
lists fidelity tier + quota headroom only. A switch that
silently drops the dev server state recreates the pain
inside Juggler. `[inferred]`

## 7. Pain cluster P6 — Wrong-account billing is silent

**What prior research said.** CUX job #1 names wrong-billing
("Start the right account on the right repo without
wrong-billing"); PROPOSAL Persona B in
[PROPOSAL.md](../../PROPOSAL.md#L52)
cites Claude #87447. Both treat it as a *selection* error
by the user.

**What external evidence adds.** The billing can be wrong
*without* any user selection error, and the UI actively
masks it — a failure class absent from prior reports:

- Headless `claude -p` spawned from an Account E session
  "authenticates as Account P — the machine-default macOS
  Keychain credential. The mismatch is silent, and
  `claude auth status` reports Account E, masking it. Usage,
  billing, and rate limits are charged to the wrong account"
  [claude-code #72552](https://github.com/anthropics/claude-code/issues/72552).
  `[verified]`
- "`/status` displays wrong account/email — session actually
  authenticated and billed against a different (Enterprise)
  org" while showing the personal account name
  [claude-code #74217](https://github.com/anthropics/claude-code/issues/74217).
  `[verified]`

| Signal | Reading |
|---|---|
| Frequency | Low–medium — needs nested/headless spawn to trigger |
| Severity | Critical — money and quota charged to the wrong Account |
| Workaround today | None; discovery is post-hoc via billing pages |
| Juggler opportunity | Per-Instance identity assertion, continuously shown |

**Feature implication.** This is the strongest argument for
the maintainer's "icons and labels — which provider,
account, models currently running" demand in
[expected-worfklow.md](../../expected-worfklow.md#L13).
Every Instance header and every spawned child must carry a
verified Account badge derived from the *effective*
credential (keychain namespace / `CODEX_HOME`), not from
the launch intent. Add a supervisor check: child process
env `CLAUDE_CONFIG_DIR`/`CODEX_HOME` must equal the
Instance's managed dir; mismatch raises a visible
wrong-account alert before first model call. `[inferred]`

## 8. Pain cluster P7 — Zombie/orphan processes and port conflicts

**What prior research said.** The maintainer workflow names
this explicitly: subprocesses "end up being zombie
processes" needing "automatic and manual clean ups, all
with verifications" in
[expected-worfklow.md](../../expected-worfklow.md#L17).
SPEC §5.1 covers re-adoption on restart in
[SPEC.md](../../SPEC.md#L224).
Prior reports do not evidence the *port* consequence.

**What external evidence adds.** The orphan's damage is a
held port discovered days later, plus parallel-agent port
fights — both with shipped community fixes Juggler should
meet or beat:

- "The model starts a dev server with `(cmd &)`, the session
  ends, and the server keeps holding its port. Days later
  you get `address already in use`, and nothing else tells
  you the server still runs" — a plugin now lists such
  servers "with their age and session and a stop button"
  [orphan-server](https://github.com/kilimcininkoroglu/claude-code-mods/blob/HEAD/plugins/orphan-server/README.md).
  `[verified]`
- Parallel worktree agents "fought over the same port";
  fix was per-worktree ports 5181–5190 plus `--strictPort`
  so "a taken port fail[s] loudly"
  [Artisan Defense field report](https://helgesver.re/articles/how-i-built-artisan-defense).
  `[verified]`
- "Two dev servers are fighting over port 3000" is a
  canonical parallel-agent failure
  [parallel agents guide](https://medium.com/ai-systems-lab/how-to-run-claude-code-agents-in-parallel-setup-guide-1d882947509e).
  `[verified]`
- A PreToolUse hook now "detects port conflicts before
  server startup… blocks the action and suggests an
  alternative" when the port belongs to another project
  [port-saver](https://github.com/gtpgg1013/port-saver).
  `[verified]`
- Orphaned plugin-cache leases from dead PIDs "pinned
  permanently" because "nothing reaps a lease whose owning
  process has exited"
  [claude-code #82145](https://github.com/anthropics/claude-code/issues/82145).
  `[verified]`

| Signal | Reading |
|---|---|
| Frequency | High — any Instance that starts a server, daily for parallel users |
| Severity | Medium–high — blocked launches, stale servers serving old code |
| Workaround today | `lsof` archaeology, orphan-server/port-saver plugins |
| Juggler opportunity | Process-tree ownership + port registry per Instance |

**Feature implication.** Juggler's supervisor already tracks
process trees (SPEC §5.2 in
[SPEC.md](../../SPEC.md#L234)).
Extend it: (a) record listening ports per Instance tree;
(b) on Instance stop, list surviving children with age and
port and offer verified cleanup (confirm PID + port freed);
(c) on launch, pre-flight the project's known dev ports and
warn on collision with another Instance. This directly
implements the maintainer's verified-cleanup demand.
`[inferred]`

## 9. Pain cluster P8 — Parallel Instances overwrite each other's work

**What prior research said.** CUX F1.1 (P0) requires
worktree isolation; the persona pain table marks worktree
collisions `† [inferred]… Needs dogfood validation`.

**What external evidence adds.** Validation now exists, and
it is worse than inferred: the failure is silent.

- "Two agents in one checkout overwrite each other's edits
  with no error and no conflict marker — git sees a single
  working copy"
  [ai-skills worktrees](https://github.com/tschallacka/ai-skills/blob/HEAD/git-worktrees/SKILL.md).
  `[verified]`
- "Do NOT let multiple agents share the main checkout —
  they will silently overwrite each other's staged changes.
  Hit this 3 times during the 2026-05-06 perf sprint"
  [field note](https://github.com/murror/murrorastro/blob/HEAD/.claude/memory/feedback_isolated_worktree_for_parallel_agents.md).
  `[verified]`
- "Multiple agents staging/committing in the same worktree
  overwrite each other" plus `index.lock` races on a shared
  `.git`
  [datus CLAUDE.md](https://github.com/datus-ai/datus-db-adapters/blob/HEAD/CLAUDE.md).
  `[verified]`

| Signal | Reading |
|---|---|
| Frequency | High for the core persona (2–3 parallel Instances per [PROPOSAL.md](../../PROPOSAL.md#L51)) |
| Severity | Critical — silent loss of agent work |
| Workaround today | Manual `git worktree add`, one branch per agent |
| Juggler opportunity | Worktree-on-launch as the parallel default |

**Correction to prior research.** Promote CUX pain #7 from
`[inferred]` to `[verified]`, and F1.1's acceptance must
include the dirty-tree case: launching a second Instance on
a project with uncommitted changes offers worktree / stash /
share-with-warning — never silent sharing. `[inferred]`

## 10. Pain clusters P9–P10 — Stuck agents, sprawl, and cost surprises

**P9: stuck/waiting Instances and session sprawl.** Prior:
CUX F2.5/F5.2 (statuses modelled, never emitted); MDR §02.
External: "The agent lives in a terminal… It stops to ask a
question while you're in a meeting, and the question sits
there for two hours"
[dev.to GetPullRequest](https://dev.to/getpullrequest-hq/your-ai-coding-agent-is-stuck-in-a-terminal-you-have-to-babysit-so-we-built-getpullrequest-1jng);
a queue tool now sorts sessions by "how long it has been
waiting… Permission prompts and questions first"
[dev.to session queue](http://dev.to/bargan/which-claude-code-session-is-waiting-on-you-a-queue-for-agents-that-stopped-to-ask-1ok9);
frozen sessions are "a stalled network connection… a tool
call stuck waiting on input… or a startup hang caused by a
slow-loading MCP server"
[Medium freeze guide](https://medium.com/@cartseoservice/claude-code-keeps-freezing-or-hanging-how-to-fix-it-5a6e07b79a85).
Frequency: daily at 5–15 Instances. Severity: high (idle
quota burn + blocked human). Workaround: terminal-tab
archaeology. **Implication:** the Needs-Attention inbox
(CUX WOW-a) must sort by waiting-time descending and show
*what* the Instance waits on (permission / question /
network), reusing SPEC stuck heuristics in
[SPEC.md](../../SPEC.md#L220).
`[verified]` evidence, `[inferred]` implication.

**P10: cost surprises from invisible token burn.** Prior:
CUX F1.2 (no USD view); CUX pain #5. External: "A default
Claude Code subagent sends 10,545 tokens before it does any
work… Three subagents cost exactly three times one"
[Plain English measurement](https://ai.plainenglish.io/every-claude-code-subagent-pays-10-545-tokens-before-working-e29d414d3a18);
"a coding agent stuck in a retry loop… can spend about
$115 an hour… a warning email arrives after that money is
gone"
[dev.to spend limits](http://dev.to/akaranjkar08/claude-code-spend-limit-hard-caps-that-stop-a-runaway-bill-1ihi);
subscription users "burn within 2 hour of coding in
interactive mode and 20 minutes if we use multiple workers"
[HN #48505268](https://news.ycombinator.com/item?id=48505268).
Frequency: high for subagent-heavy users. Severity: high.
Workaround: ccusage post-hoc reports, hard-cap scripts.
**Implication:** per-Instance live burn rate (tokens/min,
est. USD where API-billed, %/min of window where
subscription-billed) in the Instance panel header — the
right-inspector demand in
[expected-worfklow.md](../../expected-worfklow.md#L14)
— plus a passive runaway alert at 3× the Instance's median
rate (extends SPEC §5.2 anomaly rule in
[SPEC.md](../../SPEC.md#L243)
from RSS to burn). `[inferred]`

**Gemini note (missed Provider).** Paid-tier users hit
429s because the CLI "is often stuck in 'Free Tier' with
zeroed-out limits" until a separate perk is claimed
[gemini-cli #24396](https://github.com/google-gemini/gemini-cli/issues/24396);
free-tier users round-robin "multiple Google accounts…
locked out for 25 minutes"
[Medium round-robin](https://medium.com/@dasunnimantha777/how-to-get-almost-unlimited-free-gemini-access-round-robin-multiple-google-accounts-with-openclaw-d5f4afd275aa).
Same juggling shape, third Provider. SPEC adapters stop at
ZCode in
[SPEC.md](../../SPEC.md#L206);
recommend a Gemini discovery spike before claiming the
multi-Provider workflow. `[verified]`

## 11. Synthesis: what changes in priority

1. **P6 wrong-account assertion → P0.** Silent mis-billing
   with a masking UI (Claude #72552, #74217) outranks chart
   polish; it is also the clearest proof for per-Instance
   Provider/Account/model badges. Extends CUX job #1.
2. **P8 worktree guard: inferred → verified P0.** Keep CUX
   F1.1; add the dirty-tree launch decision. No new scope.
3. **P4 shared-settings copy → P1** (from CUX F1.9 P2):
   second-Account usability blocker, not a nicety.
4. **P2 binding-window quota line → P0 display rule** for
   M3: headroom + binding window + absolute reset, always.
   Sharpens MDR §06 honesty tiers with a layout mandate.
5. **P7 port registry + verified cleanup → M2**, beside
   stuck detection (EPICS M2-5/M2-6): the maintainer's
   zombie demand with external proof of the port cost.
6. **No priority change:** handover tiers (MDR §09) and
   guardrails (PROPOSAL §5) stand; external switchers
   confirm the user-clicked pool view (§4) as the compliant
   way to match their speed. `[inferred]`

## 12. Search log (20 queries, 2026-10-13)

Quota/walls: `Claude Code rate limit quota exhausted mid-task Reddit`; `Codex CLI rate limit weekly quota hit wall
Hacker News`; `"rate limit" reset timing 5h window Claude Code reset confusion`. Switching:
`Claude Code CLAUDE_CONFIG_DIR multiple accounts switching
pain`; `multiple Claude subscriptions Max account switching
hack alias workaround GitHub`; `Hacker News Claude Code
multiple subscriptions juggling accounts discussion`;
`Codex CLI session resume context lost switching accounts
CODEX_HOME`; `wrong account billing Claude Code work
personal account charged GitHub issue`. Rules:
`AI coding agent context loss switching tools CLAUDE.md
AGENTS.md duplication`; `MCP config duplication per account Claude Code`. Processes:
`Claude Code zombie orphan processes dev server port
conflict left running`; `Claude Code stuck agent hanging session sprawl`; `parallel
AI agents git worktree branch conflict same repo
overwrite`. Cost/other Providers: `Claude Code cost
surprise token burn expensive subagents Reddit`; `Gemini CLI quota multiple accounts forum`.
Plus 5 follow-up page reads counted in the same pass. Unresolved: no live Reddit thread bodies were fetched
(search snippets only); Gemini ZCode-parity quota API
unverified; Codex Estimated-window accuracy untested
against a live account — flagged `[inferred]` where relied on.





