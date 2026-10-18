# Deep competitor review 04 — T3 Code & Opcode vs AI Juggler
>
> Date: 2026-10-13. Scope: T3 Code (`pingdotgg/t3code`,
> [t3.codes](https://t3.codes/)) and Opcode, formerly Claudia
> (`winfunc/opcode`, [opcode.sh](https://opcode.sh/)).
> Method: official site/docs/GitHub/README + release/issue primary
> sources via web search, cross-checked against the parent review
> [full-competitive-ux-review.md](../full-competitive-ux-review.md#L522)
> and Juggler source. Every competitor claim carries a URL; every
> Juggler claim carries `file:line`. Star counts are snapshot-only.
> Prior review lines cited below: T3 at
> [full-competitive-ux-review.md](../full-competitive-ux-review.md#L522),
> Opcode at
> [full-competitive-ux-review.md](../full-competitive-ux-review.md#L545).
> This file goes deeper on history, onboarding, execution model,
> review loop, and churn; it does not restate the parent summary.
>
> Verdict keys: **STEAL** = copy the pattern, **BEAT** = Juggler can
> win by doing the honest/juggling version, **AVOID** = do not copy.
> Gap severity: **P0** blocks Juggler's core promise, **P1** before
> v0.1-beta, **P2** later. Negative evidence is marked **[NEG-EV]**.
>
> Juggler anchors used throughout: only Claude adapter registered
> ([state.rs](../../../crates/juggler-app/src/state.rs#L91)),
> adapter trait at
> ([adapter.rs](../../../crates/juggler-adapter/src/adapter.rs#L26)),
> quota model at
> ([model.rs](../../../crates/juggler-core/src/model.rs#L277)),
> Usage/Fleet still `ComingSoon`
> ([App.tsx](../../../ui/src/App.tsx#L339)).
---

## 1. Executive judgment (read this first)

- T3 and Opcode attack opposite ends: T3 is a *multi-harness
  control surface* with mobile/remote reach
  ([README](https://github.com/pingdotgg/t3code/blob/main/README.md));
  Opcode is a *single-harness depth tool* (checkpoints, agents,
  MCP, usage) for Claude Code only
  ([README](https://github.com/winfunc/opcode/blob/HEAD/README.md)).
- Neither owns multi-account juggling. T3 uses provider instances,
  not named account profiles — parent finding at
  [full-competitive-ux-review.md](../full-competitive-ux-review.md#L475);
  Opcode multi-account is **[NEG-EV]** — no feature found in
  README/docs, per
  [full-competitive-ux-review.md](../full-competitive-ux-review.md#L554).
- Juggler's wedge is therefore intact but time-boxed: T3 ships
  worktrees, diff, PR linking, palette, and remote today, while
  Juggler's quota UI is hardcoded `percent={0}`
  ([App.tsx](../../../ui/src/App.tsx#L513)).
- Opcode proves demand for checkpoints/agents/MCP depth, then
  stalls — last dated commits Oct 2025
  ([commit](https://github.com/winfunc/opcode/commit/70c16d8a4910db48cd9684aeacdd431caefd7d71)).
  Its users are migratable, but its AGPL depth is not copyable
  wholesale (see §7).
- The single most stealable ideas: T3's thread/worktree/PR
  lifecycle and ⌘K palette; Opcode's checkpoint-fork and MCP
  registry UX. The single most beatable gap: honest quota —
  both competitors blur cost vs remaining subscription quota.

## 2. T3 Code — history, team, traction, funding

### 2.1 Origin and timeline

- Repo `pingdotgg/t3code`, created 2026-02-08, by Theo Browne /
  T3 Tools Inc. (Ping.gg)
  ([research mirror](https://github.com/2002bishwajeet/yantra/blob/HEAD/docs/research/15-t3code-for-the-chat.md)).
- Public launch 2026-03-07: "T3 Code is now available for
  everyone… Fully open source"
  ([launch summary](https://github.com/seth-torrence/artemis/blob/HEAD/docs/research/T3-DESIGN-RESEARCH.md)).
- Positioned in its own `AGENTS.md` as an open-source
  "bring-your-own-subscription" alternative to Claude Desktop,
  Codex App, Cursor Glass and Conductor
  ([AGENTS.md](https://github.com/jaiir320/t3code/blob/HEAD/AGENTS.md)).
- Self-described as an "agent harness control surface" for
  agents already on your machine — web, Electron desktop, and
  mobile clients
  ([README](https://github.com/pingdotgg/t3code/blob/main/README.md)).
- Release cadence is fast/nightly: `v0.0.42` stable plus nightly
  builds through Oct 2026
  ([release v0.0.42](https://github.com/pingdotgg/t3code/releases/tag/v0.0.42)).

### 2.2 Team and funding

- Built by Theo Browne (t3.gg, T3 Stack, T3 Chat) and the T3
  Tools team incl. Julius Marminge
  ([contributor note](https://github.com/lachlanstuart/l6claw)).
- **[NEG-EV]** No venture funding announcement for T3 Code
  itself was found in official sources searched; Ping.gg/T3
  Chat is the operating company
  ([README](https://github.com/pingdotgg/t3code/blob/main/README.md)).
  Do not cite a raise without a primary source.
- Monetisation stance is explicitly anti-markup: "no paid tier
  and will not monetize usage… bring your own subscription"
  ([analysis](https://enterprisedna.co/resources/ai-pulse/ai-pulse-2026-08-08-t3-code-doubles-down-on-zero-markup-pricing-as-a-trust-play/)).
- That stance is a trust play, not a business model — it
  pressures every paid orchestrator (cf. Conductor pricing in
  parent review
  [full-competitive-ux-review.md](../full-competitive-ux-review.md#L625)).

### 2.3 Traction (snapshot, not ranking)

- ~21,893 stars / 5,385 forks at 2026-09-07 snapshot, latest
  release `v0.0.39` then
  ([research mirror](https://github.com/2002bishwajeet/yantra/blob/HEAD/docs/research/15-t3code-for-the-chat.md)).
- Parent review snapshot: ~24k stars (2026-09-24), "alpha
  data-loss ToS, poor CLI sync" as weaknesses
  ([full-competitive-ux-review.md](../full-competitive-ux-review.md#L543)).
- AGENTS.md-derived claim of ">200k users" circulates via
  mirrors — treat as vendor claim, not audited metric
  ([research](https://github.com/seth-torrence/artemis/blob/HEAD/docs/research/T3-DESIGN-RESEARCH.md)).
- Fork ecosystem is itself a traction signal: ACP-provider
  forks, TTS forks, TUI clients all track upstream weekly
  ([fork example](https://github.com/jarmen423/t3code)).
- **Implication for Juggler:** T3's distribution (Theo's
  audience + `npx t3`) is the moat, not any single feature.
  Juggler cannot out-shout it; it must out-juggle it.

## 3. T3 Code — product deep dive

### 3.1 Install and onboarding

- Zero-install trial: `npx t3@latest` (Node 22.16+/23.11+/
  24.10+) starts backend + local web app
  ([README via fork](https://github.com/palenza/raptor)).
- If Claude Code/Codex is already authenticated, first session
  is near-instant — "begin using the interface almost
  immediately" ([stealwhatworks](https://stealwhatworks.com/blogs/news/why-t3-code-growing-so-fast)).
- Per-provider auth is delegated: `claude auth login`,
  `agent login` (Cursor), `grok login`, `opencode auth login`
  ([README](https://github.com/kossnocorp/t3code)).
- Desktop (Electron), hosted web `app.t3.codes`, iOS/Android
  apps all front the same local server
  ([README](https://github.com/barretodiego/t3code/blob/HEAD/README.md)).
- **Vs Juggler:** Juggler's onboarding wizard is the parent
  review's "best-built surface"
  ([full-competitive-ux-review.md](../full-competitive-ux-review.md#L213)).
  T3 wins on *time-to-first-token* (`npx` one-liner); Juggler
  wins on *account hygiene* (scan/import/login guidance).
  Verdict: **STEAL** the one-command trial path for beta.

### 3.2 Supported harnesses, models, accounts

- Harnesses: Claude Code, Codex, Cursor, Grok Build, OpenCode,
  Google Antigravity
  ([README](https://github.com/pingdotgg/t3code/blob/main/README.md)).
- Independent comparison confirms Claude/Codex/OpenCode/Cursor
  today ([compare](https://github.com/kogan/superset/blob/HEAD/apps/marketing/content/compare/superset-vs-t3-code.mdx)).
- Models are picked per provider from that provider's catalog;
  forks add "full model catalog, slash commands, approval
  modes" via ACP — evidence the core picker is extensible
  ([fork](https://github.com/jarmen423/t3code)).
- Accounts: **[NEG-EV for named profiles]** — multi-account is
  provider *instances* / env auth, not first-class named
  profiles (parent finding
  [full-competitive-ux-review.md](../full-competitive-ux-review.md#L530)).
- Continuation/resume compatibility "varies by harness"
  ([full-competitive-ux-review.md](../full-competitive-ux-review.md#L532)).
- **Vs Juggler:** Juggler has exactly one registered adapter
  ([state.rs](../../../crates/juggler-app/src/state.rs#L91))
  but first-class `Profile = account + settings`
  ([model.rs](../../../crates/juggler-core/src/model.rs#L173)).
  Breadth: T3 wins 6-to-1. Depth of account model: Juggler
  wins. Verdict: **BEAT** on accounts, **AVOID** chasing 6
  harnesses before quota (parent F6.1).

### 3.3 Project / session / terminal model

- Core unit is the *thread* inside a project; a Node
  WebSocket server wraps provider CLIs and serves all clients
  ([AGENTS.md](https://github.com/jaiir320/t3code/blob/HEAD/AGENTS.md)).
- Thread sidebar tracks PR per unsettled thread from its saved
  branch/worktree, with manual link/unlink override
  ([thread-sidebar](https://github.com/9cb14c1ec0/t3code/blob/HEAD/docs/user/thread-sidebar.md)).
- Terminal history is capped at 5,000 lines, control sequences
  stripped, one log per terminal, reloaded on first open
  ([terminal research](https://github.com/ticketry-hq/ticketry/blob/HEAD/docs/research/t3-code-terminals.md)).
- Lifecycle vocabulary: settle / unsettle / snooze / interrupt
  / delegate ([t3code-threads skill](https://github.com/arrrnmp/t3code-tui/blob/HEAD/skills/use-t3code-cli/SKILL.md)).
- Settlement is persisted server-side; clients render it, and
  a `thread.settled` event can stop an idle provider session
  ([internals](https://github.com/ochen1/t3dpcode/blob/HEAD/docs/internals/overview.md)).
- **Vs Juggler:** Juggler sessions carry only `cwd` + profile
  ([model.rs](../../../crates/juggler-core/src/model.rs#L394));
  rail caps at `slice(0,15)` with no search
  ([WorkflowRail.tsx](../../../ui/src/components/WorkflowRail.tsx#L377)).
  Verdict: **STEAL** settle/archive + snooze semantics (P1).

### 3.4 Parallel execution and worktrees

- One task/thread = one git worktree + branch is the headline
  parallel workflow ("worktree-based parallel project
  workflow… standout" [bestofai](https://bestofai.com/our-tool-picks)).
- Atomic thread bootstrap for new worktrees (`--checkout
  worktree`): create thread, prepare worktree, run setup
  script, start prompt
  ([t3code-tui](https://github.com/arrrnmp/t3code-tui/blob/HEAD/README.md)).
- `newWorktreesStartFromOrigin` config controls base ref;
  default flipped `false`→`true` in 0.0.29
  ([t3code-tui](https://github.com/arrrnmp/t3code-tui/blob/HEAD/README.md)).
- Orchestrator pattern: workers in worktrees, "preserve
  history and worktrees unless deletion explicitly authorized"
  ([orchestrator prompt](https://github.com/gfsaaser24/t3-code-agent-mcp/blob/HEAD/examples/orchestrator-prompt.md)).
- **Vs Juggler:** no worktree/branch concept outside a pack
  field — parent F1.1
  ([full-competitive-ux-review.md](../full-competitive-ux-review.md#L63)).
  This is a **P0 gap**: parallel accounts on one repo without
  isolation will collide. Verdict: **STEAL** (P0/P1).

### 3.5 Diff, review, and PR flow

- Diff viewer with staging/commit per thread (parent review
  [full-competitive-ux-review.md](../full-competitive-ux-review.md#L536)).
- Sidebar auto-links the PR for each unsettled thread's branch;
  right-click to link a different PR or unlink
  ([thread-sidebar](https://github.com/9cb14c1ec0/t3code/blob/HEAD/docs/user/thread-sidebar.md)).
- Orchestrator guidance: "Settle completed PRs and archive
  finished worker threads… Do not archive active review
  owners" ([prompt](https://github.com/gfsaaser24/t3-code-agent-mcp/blob/HEAD/examples/orchestrator-prompt.md)).
- **[NEG-EV]** Inline comment → agent round-trip (Vibe Kanban's
  signature) was not found in official T3 docs searched.
- **Vs Juggler:** no diff/review loop at all (parent F7.3
  [full-competitive-ux-review.md](../full-competitive-ux-review.md#L497)).
  Verdict: **STEAL** read-only diff + PR link first (P1);
  **AVOID** full staging/commit until review demand is proven.

### 3.6 Agents, subagents, and delegation

- Threads support delegate/interrupt/snooze operations exposed
  to external orchestrators via CLI skills
  ([skill](https://github.com/arrrnmp/t3code-tui/blob/HEAD/skills/use-t3code-cli/SKILL.md)).
- A `t3code-handoff` skill hands a conversation to a fresh
  thread ([skill index](https://github.com/arrrnmp/t3code-tui/blob/HEAD/skills/use-t3code-cli/SKILL.md)).
- Community MCP bridge (`t3-code-agent-mcp`) orchestrates
  worker threads externally — evidence T3 is scriptable, not
  that subagents are a native first-class entity
  ([repo](https://github.com/gfsaaser24/t3-code-agent-mcp/blob/HEAD/examples/orchestrator-prompt.md)).
- **[NEG-EV]** No native named subagent library/permissions
  model like Opcode's was found in official T3 README/docs.
- **Vs Juggler:** Juggler has no agent/subagent concept; its
  adapter trait is launch/watch only
  ([adapter.rs](../../../crates/juggler-adapter/src/adapter.rs#L26)).
  Verdict: **BEAT** via handover lineage, not agent library.

### 3.7 MCP support

- **[NEG-EV]** Official T3 README positions MCP as something
  third parties build *on* T3 (agent-mcp bridge), not a
  built-in MCP registry UI
  ([bridge](https://github.com/gfsaaser24/t3-code-agent-mcp/blob/HEAD/examples/orchestrator-prompt.md)).
- Provider-level MCP/skills pass through the underlying CLIs;
  forks surface provider slash commands/approvals
  ([fork](https://github.com/jarmen423/t3code)).
- **Vs Juggler:** no MCP sync/portability (parent F1.9
  [full-competitive-ux-review.md](../full-competitive-ux-review.md#L148)).
  Both are absent; Opcode (not T3) is the MCP benchmark.
  Verdict: **AVOID** racing T3 here; learn from Opcode §5.7.

### 3.8 Usage, quota, and cost

- A usage panel shows token + API-cost analytics (parent
  [full-competitive-ux-review.md](../full-competitive-ux-review.md#L537)).
- Whether it reflects *official remaining subscription quota*
  is disputed; a 5h/weekly probe exists only in forks
  ([full-competitive-ux-review.md](../full-competitive-ux-review.md#L538)).
- Independent quota matrix scores T3 official quota % as
  absent (❌), reset countdowns ❌
  ([matrix](https://github.com/superset-sh/superset/blob/HEAD/plans/20260816-quota-tools-marketing-research.md)).
- Internal usage design tracks `total_cost_usd` passthrough
  with "no cost tables" plus context/cache counters
  ([DECOUPLE](https://github.com/arrrnmp/t3code-tui/blob/HEAD/DECOUPLE.md)).
- Pricing posture: free, zero markup, "exactly what they pay"
  as trust wedge
  ([analysis](https://enterprisedna.co/resources/ai-pulse/ai-pulse-2026-08-08-t3-code-doubles-down-on-zero-markup-pricing-as-a-trust-play/)).
- **Vs Juggler:** Juggler models `QuotaWindow` + honesty tiers
  Measured/Computed/Estimated in SPEC
  ([SPEC.md](../../SPEC.md#L256))
  but UI rings are hardcoded zero
  ([App.tsx](../../../ui/src/App.tsx#L513)).
  Verdict: **BEAT** — ship Measured-labelled quota; never blur
  cost into quota the way the disputed T3 panel does (P0).

### 3.9 Notifications, remote, and mobile

- Remote session control: pair phone/tablet via QR/pairing
  token or hosted `app.t3.codes`; server is local Node,
  optionally LAN/Tailscale/SSH
  ([ade-arena](https://github.com/fcamblor/ade-arena/issues/4)).
- `REMOTE.md`-backed pairing is a documented feature, not a
  hack ([summary](https://github.com/fcamblor/ade-arena/issues/4)).
- Mobile apps (iOS/Android) are first-class store listings
  ([README](https://github.com/barretodiego/t3code/blob/HEAD/README.md)).
- Background-service mode and remote-access docs ship in-repo
  ([docs index](https://github.com/jarmen423/t3code)).
- **[NEG-EV]** Granular notification policy (quiet digest,
  per-project mute) was not found in official docs searched.
- **Vs Juggler:** Fleet is `ComingSoon`
  ([App.tsx](../../../ui/src/App.tsx#L341));
  tray is specced quota-only
  ([SPEC.md](../../SPEC.md#L352)).
  Verdict: **STEAL** QR pairing model for Fleet (P2);
  **BEAT** on notification discipline (WOW-g, parent §UX).

### 3.10 Search, history, keyboard, palette

- ⌘K command palette searches commands, projects, threads;
  `>` prefix filters to actions; ⌘1–⌘9 jump to threads
  ([keybindings](https://github.com/onkarj012/t3code/blob/HEAD/docs/user/keybindings.md)).
- iPad hardware-keyboard shortcuts + palette shipped in
  v0.0.42 ([release](https://github.com/pingdotgg/t3code/releases/tag/v0.0.42)).
- Keybindings are user-editable via Settings → Keybindings,
  persisted to `~/.t3/userdata/keybindings.json`
  ([keybindings](https://github.com/dbalders/tritonai-harness/blob/HEAD/docs/user/keybindings.md)).
- Import of external Claude Code/Codex conversations is a
  documented user flow
  ([docs index](https://github.com/ochen1/t3dpcode/blob/HEAD/docs/README.md)).
- CLI-sync gap is the known wound: resume picker for external
  sessions requested (#5346), `/resume` broken in CLI-used
  folders (#4519)
  ([discussion #6994](https://github.com/pingdotgg/t3code/discussions/6994)).
- **Vs Juggler:** no palette/search; rail truncated lists only
  ([WorkflowRail.tsx](../../../ui/src/components/WorkflowRail.tsx#L377)).
  Verdict: **STEAL** ⌘K wholesale (P0 WOW-c in parent review).

### 3.11 Settings, platforms, pricing, licence

- Settings surface includes keybindings, project icons,
  permission modes, telemetry opt-out, mobile appearance
  ([docs index](https://github.com/ochen1/t3dpcode/blob/HEAD/docs/README.md)).
- Anonymous usage telemetry exists and is documented — note
  the contrast with Juggler's local-first/no-telemetry stance
  ([README.md](../../../README.md#L38)).
- Platforms: Electron desktop + web + iOS + Android, one
  server ([README](https://github.com/pingdotgg/t3code/blob/main/README.md)).
- Price: free; licence MIT (T3 Tools Inc.)
  ([compare](https://github.com/kogan/superset/blob/HEAD/apps/marketing/content/compare/superset-vs-t3-code.mdx)).
- Maturity warning from comparators: "very early alpha,
  expect rapid change"
  ([compare](https://github.com/kogan/superset/blob/HEAD/apps/marketing/content/compare/superset-vs-t3-code.mdx)).
- Known churn drivers: V2 closing sessions mid-batch (#16697),
  T3 Connect failures on Windows (#14070, #14836), Codex
  plugin blindness (#9044)
  ([issue #16697](https://github.com/pingdotgg/t3code/issues/16697),
  [issue #14070](https://github.com/pingdotgg/t3code/issues/14070)).
- **Lesson:** velocity + audience hides alpha breakage for
  T3; Juggler (no audience buffer) cannot afford the same
  data-loss reputation. Verdict: **AVOID** alpha-ToS posture.

## 4. Opcode — history, team, traction, funding

### 4.1 Origin: Claudia → Opcode

- Repo `winfunc/opcode` (org also seen as `getAsterisk`),
  created 2025-06-19, by Asterisk Labs ("Made with ❤️ by
  Asterisk" [README](https://github.com/winfunc/opcode)).
- Formerly **Claudia**; rebranded to Opcode mid-2025
  ([competitive note](https://github.com/hamed-farag/claudeck/blob/HEAD/docs/COMPETITIVE-ANALYSIS.md)).
- Company described as Y Combinator-backed Asterisk Labs
  ([research](https://github.com/michaellinhardt/grimoire/blob/HEAD/_bmad-output/research/opcode-backend-ai-features.md)).
- Tauri 2 + React + TypeScript + Rust desktop app wrapping
  the Claude Code CLI
  ([landscape](https://github.com/roman10/mcode/blob/HEAD/docs/open-source/competitive-landscape.md)).
- Data model: spawns `claude` with stream-json and parses
  `~/.claude/projects/*.jsonl`
  ([research](https://github.com/enerage/vibespace/blob/HEAD/RESEARCH-CLAUDE-DATA.md)).

### 4.2 Traction and dormancy

- ~21–22k stars, ~1.7k forks, AGPL-3.0 (snapshots vary)
  ([facts](https://agentskillshub.top/skill/winfunc/opcode/),
  [landscape](https://github.com/farol-team/workroom/blob/HEAD/docs/spikes/agent-ui-landscape.md)).
- Parent review: "~21-22k stars… dormant since Oct 2025,"
  strengths checkpoints/MCP/agents
  ([full-competitive-ux-review.md](../full-competitive-ux-review.md#L575)).
- Last dated commits cluster 2025-10-05…2025-10-13 (edition
  fix, wss fix) ([commit](https://github.com/winfunc/opcode/commit/70c16d8a4910db48cd9684aeacdd431caefd7d71)).
- 332 open issues at snapshot; build-broken reports on the
  last commit (#369) are a churn accelerant
  ([issue #369](https://github.com/winfunc/opcode/issues/369),
  [facts](https://agentskillshub.top/skill/winfunc/opcode/)).
- Discovery chain explicitly flags it "largest-by-star Claude
  Code GUI" *and* stale in the same breath
  ([discovery](https://github.com/julo15/seshctl/blob/HEAD/.agents/discoveries/2026-05-25-1700-seshctl-alternatives.md)).
- **Implication:** Opcode validates the GUI-wrapper market,
  then vacates it. Its feature checklist is a free product
  spec; its maintenance vacuum is Juggler's opening (same
  pattern as Vibe Kanban/Crystal, parent F7.4).

## 5. Opcode — product deep dive

### 5.1 Install and onboarding

- First run auto-detects `~/.claude`; prerequisites are
  Rust/bun/git per-platform per README install section
  ([README](https://github.com/winfunc/opcode)).
- No `npx`-style one-liner; build-from-source friction is
  documented in issue #369's clone/bun/tauri-build path
  ([issue #369](https://github.com/winfunc/opcode/issues/369)).
- **Vs Juggler:** Juggler's wizard (scan → read-only import →
  guided vendor login) is strictly better for multi-account
  users (parent
  [full-competitive-ux-review.md](../full-competitive-ux-review.md#L215)).
  Verdict: **BEAT** — keep wizard; never regress to
  detect-one-dir onboarding.

### 5.2 Harnesses, models, accounts

- Single-harness by design: Claude Code only
  ([full-competitive-ux-review.md](../full-competitive-ux-review.md#L554)).
- Models: per-agent model choice among Claude models
  ([README mirror](https://github.com/khj-nextground/opcode)).
- Accounts: **[NEG-EV]** no multi-account feature in
  README/docs; swapping means OS/CLI-level `~/.claude` swap
  ([full-competitive-ux-review.md](../full-competitive-ux-review.md#L556)).
- **Vs Juggler:** Juggler's whole thesis (named profiles,
  isolated config dirs) is the feature Opcode lacks.
  Verdict: **BEAT** (core wedge, P0 to make visible).

### 5.3 Project / session / terminal model

- Visual project → session browser over
  `~/.claude/projects/`, with search and resume
  ([research](https://github.com/enerage/vibespace/blob/HEAD/RESEARCH-CLAUDE-DATA.md)).
- Multiple interactive sessions; CLAUDE.md editor built in
  ([landscape](https://github.com/roman10/mcode/blob/HEAD/docs/open-source/competitive-landscape.md)).
- Experimental web-server mode for mobile access exists in
  analysis forks ([analysis](https://github.com/canyavall/nori/blob/HEAD/analysis/opcode/README.md)).
- **Vs Juggler:** Juggler embeds live PTY terminals per
  session; Opcode centres the chat/session browser. Both
  parse the same JSONL. Verdict: **STEAL** project→session
  two-level browsing clarity (P2).

### 5.4 Parallel execution and worktrees

- Parallelism = multiple interactive/background sessions,
  not worktree isolation. **[NEG-EV]** no worktree/kanban
  orchestration layer
  ([research](https://github.com/3esign/coddess/blob/HEAD/docs/research-track2.md)).
- Background agents run sandboxed (seccomp/Seatbelt per
  parent [full-competitive-ux-review.md](../full-competitive-ux-review.md#L564)).
- **Vs Juggler:** neither has worktrees today; T3 does.
  Verdict: **AVOID** copying session-only parallelism —
  pair Juggler parallelism with worktrees (F1.1, P0).

### 5.5 Diff, review, PR

- Diff viewer *between checkpoints* (not branch/PR diff)
  ([README](https://github.com/winfunc/opcode/blob/HEAD/README.md)).
- **[NEG-EV]** no PR creation/branch-review flow found in
  official README/docs searched.
- **Vs Juggler:** both lack PR flow; T3/Conductor own it.
  Verdict: **STEAL** checkpoint-diff *idea* as session-replay
  diff (WOW-i, P2), not as the review queue.

### 5.6 Agents and subagents (Opcode's signature depth)

- Custom agents: system prompt, model choice, file/network
  permissions, agent library, import/export `.opcode.json`
  ([README](https://github.com/winfunc/opcode/blob/HEAD/README.md),
  [mirror](https://github.com/khj-nextground/opcode)).
- Secure background agents with sandboxing (parent
  [full-competitive-ux-review.md](../full-competitive-ux-review.md#L562)).
- Third-party notices credit opcode for "MCP integration,
  custom agents, checkpoint/timeline, zstd+SHA-256 storage"
  ([notices](https://github.com/soul667/muxfleet/blob/HEAD/THIRD_PARTY_NOTICES.md)).
- **Vs Juggler:** Juggler deliberately supervises harnesses,
  not agent personas
  ([README.md](../../../README.md#L36)).
  Verdict: **STEAL** import/export-shareable-config pattern
  for Juggler *profiles* (F1.9, P1); **AVOID** agent library.

### 5.7 MCP management

- MCP server registry/management UI; import server configs
  from Claude Desktop
  ([README](https://github.com/winfunc/opcode/blob/HEAD/README.md)).
- "Better visualization of MCP responses" in execution UI
  ([README mirror](https://github.com/VasilyPolyuhovich/opcode)).
- **Vs Juggler:** zero MCP surface; only specced profile sync
  (parent F1.9). Verdict: **STEAL** registry + Desktop-import
  UX when M-sync lands (P1/P2). This is Opcode's most
  transferable feature.

### 5.8 Timeline and checkpoints (the feature to respect)

- Create a checkpoint at any point; branching visual
  timeline; one-click restore; fork sessions from any
  checkpoint; diff between checkpoints
  ([README](https://github.com/winfunc/opcode/blob/HEAD/README.md)).
- Parent verdict: "Nothing in Juggler's SPEC matches this;
  closest is the handover pack (different problem)"
  ([full-competitive-ux-review.md](../full-competitive-ux-review.md#L572)).
- Checkpoints solve *time travel within* a session; handover
  solves *moving across* accounts. Users conflate them.
- **Vs Juggler:** handover v0 is transcript byte-copy +
  `--resume` (parent F3.3). Verdict: **STEAL** fork-from-
  point interaction for WOW-i replay (P2); **BEAT** by
  labelling handover vs checkpoint honestly in UI copy.

### 5.9 Usage, quota, cost

- Usage Dashboard: cost by model/project/date, token charts,
  export; alerts "coming soon" (parent
  [full-competitive-ux-review.md](../full-competitive-ux-review.md#L566)).
- It is *cost analytics from JSONL*, not remaining
  subscription quota — same blur as T3, honestly labelled
  or not. **[NEG-EV]** no 5h/weekly remaining-quota feed
  found in official docs.
- **Vs Juggler:** Juggler SPEC promises Measured quota via
  statusline feed
  ([SPEC.md](../../SPEC.md#L192))
  — strictly stronger claim, currently unbuilt. Verdict:
  **BEAT** (P0): ship quota+cost as separate labelled views;
  **STEAL** cost-by-project export shape for WOW-e (P1).

### 5.10 Search, keyboard, settings, platform, licence

- Session search in the browser; slash-command picker with
  cross-command search (`/clear` gap fixed via #338 fork)
  ([commit](https://github.com/lusipad/claudia/commit/5cf9957529c20890623b37a642d6857a88648e90)).
- **[NEG-EV]** no ⌘K global palette / editable keybindings
  found in official README/docs (contrast T3 §3.10).
- Platforms macOS/Linux/Windows; licence AGPL-3.0, free
  ([facts](https://agentskillshub.top/skill/winfunc/opcode/)).
- AGPL matters: Juggler is MIT
  ([README.md](../../../README.md#L83)).
  Copy *ideas*, never Opcode code, into Juggler. **AVOID**.
- Churn reasons: dormancy + broken HEAD build + 332 open
  issues ([issue #369](https://github.com/winfunc/opcode/issues/369)).
  Users don't churn *to* a feature; they churn *from*
  abandonment. Verdict: **BEAT** by simply shipping.

## 6. Feature-by-feature: Them vs AI Juggler

| Feature | Them (T3 / Opcode) | Us | Gap | Verdict |
|---|---|---|---|---|
| Worktree isolation | T3 yes; Opcode no [NEG-EV] | Absent (F1.1) | P0 | STEAL T3 |
| Named multi-account | Neither (instances/dir-swap) | Built profiles | — (lead) | BEAT |
| Quota remaining | Neither (cost only) | Planned M3, UI zero | P0 | BEAT |
| Cost USD | Both: cost analytics | Absent (F1.2) | P1 | STEAL shape |
| Diff/PR | T3 PR-link; Opcode ckpt-diff | Absent | P1 | STEAL T3 |
| Checkpoints/fork | Opcode signature | Absent (pack only) | P2 | STEAL idea |
| Agents library | Opcode yes; T3 no [NEG-EV] | Absent by design | P2 | AVOID |
| MCP registry | Opcode yes; T3 no [NEG-EV] | Absent | P1 | STEAL Opcode |
| ⌘K palette | T3 yes; Opcode no [NEG-EV] | Absent | P0 | STEAL T3 |
| Mobile/remote | T3 store apps+QR | Fleet ComingSoon | P2 | STEAL QR |
| Handover lineage | T3 thread-handoff skill | v0 transcript-only | P0 | BEAT honest |
| Telemetry | T3 anonymous opt-out | None (local-first) | — (lead) | BEAT |

- Row evidence: T3 worktree/PR/palette §3.4/§3.5/§3.10;
  Opcode checkpoints/agents/MCP §5.6–§5.8; Juggler gaps at
  parent F1.1/F1.2/F1.9 and
  ([App.tsx](../../../ui/src/App.tsx#L339)).
- Juggler capabilities advertised (`TRANSPLANT|FORK`) exceed
  implementation — downgrade per parent F3.1
  ([claude_code.rs](../../../crates/juggler-adapters/src/claude_code.rs#L111)).

## 7. Strengths, weaknesses, churn — synthesis

- **T3 strengths:** breadth (6 harnesses), `npx` trial,
  worktree/PR thread model, mobile reach, MIT + zero-markup
  trust. **Weaknesses:** alpha breakage (#16697/#14070),
  CLI-sync wound (#6994), quota blur (§3.8), telemetry.
  **Churn trigger:** data-loss/session-kill bugs, not price.
- **Opcode strengths:** checkpoint timeline, agents+MCP
  depth, cost dashboard, polished single-harness UX.
  **Weaknesses:** Claude-only, no accounts/worktrees,
  AGPL, dormant since Oct 2025 with broken HEAD (#369).
  **Churn trigger:** abandonment itself.
- **Juggler's play:** take T3's lifecycle + palette, Opcode's
  MCP/config portability, beat both on the one axis neither
  attempts — named-account, honesty-labelled quota juggling
  ([SPEC.md](../../SPEC.md#L259)).
  Sequence per parent Phase 1: quota + needs-attention +
  honest handover before harness #2. Anything else is
  building Opcode's checklist while T3 eats the market.

## Appendix — unresolved / negative evidence log

- T3 funding: no primary raise found **[NEG-EV]** (§2.2).
- T3/Opcode notification policy depth: not in official docs.
- Live star counts: snapshot mirrors only, no GitHub API
  fetch this session (parent Appendix A caveat applies).
- In-app (non-doc) settings for both tools were not run;
  absence claims are docs/README-level **[NEG-EV]** only.
