# 06 — Vibe, Crystal & the Switchers: quota/switching table stakes

> Section 06 of the deep review. Parent: `docs/review/full-competitive-ux-review.md`.
> Question: what do Vibe Kanban, Crystal, cc-switch, ccusage,
> claude-swap/`cswap`, `aisw`, and the native Claude Code / Codex
> usage surfaces actually do — and which of their behaviours are
> day-1 table stakes for Juggler's juggling promise?
> Method: competitor web research (URLs inline) + repo inspection
> (`file:line`). No live competitor install was run; install-time
> claims are labelled `[docs]` vs `[observed-in-repo]`.

## Verdict in five lines

- Orchestrators (Vibe, Crystal) solved parallel worktrees, never quota/rotation; their deaths orphan *parallel-session* users, so winning them needs worktrees + diff review first.
- Switchers (`cc-switch`, `cswap`, `aisw`, `ccusage`) solved quota visibility + one-click switching, but none supervises live sessions (no stuck/lineage/handover).
- Native baselines rose in 2026 (statusline `rate_limits`, Codex `/status`); any Juggler quota UI less truthful than `/usage` will be judged against `/usage`.
- Juggler advertises unimplemented quota/handover: `USAGE_QUOTA_FEED|TRANSPLANT|FORK` in [claude_code.rs](../../../crates/juggler-adapters/src/claude_code.rs#L111) vs `percent={0}` rings + `ComingSoon` Usage in UI.
- The open niche is *quota-aware session supervision*, won on §8's day-1 behaviours — not harness breadth.

## 1. How to read this section

### 1.1 Honesty tiers (Juggler's own vocabulary)

Juggler models trust explicitly: `Confidence::{Measured, Computed,
Estimated}` with `Estimated` as the safe default in
[model.rs](../../../crates/juggler-core/src/model.rs#L286).
The spec's honesty rule — "every number in the UI carries its
confidence" — is stated in
[SPEC.md](../../SPEC.md#L254).
This section grades every competitor number against the same tiers:

- **Measured** — vendor-reported %/reset (statusline feed, `/usage`,
  OAuth usage API, Codex `token_count` snapshots).
- **Computed** — deterministic local math (JSONL tokens × price).
- **Estimated** — inferred burn vs an assumed plan cap.

Most switcher marketing collapses these three into one number.
Juggler's differentiation is to *not* collapse them — if the UI
keeps the labels honest.

### 1.2 What "switching" means (four different verbs)

| Verb | Meaning | Who does it |
|---|---|---|
| Config switch | Rewrite live config to another provider/key | cc-switch |
| Account switch | Swap the active login credential set | cswap, aisw |
| Session transplant | Move one live conversation to another account | Juggler (specced) |
| Auto-rotation | Switch without asking when quota crosses a line | cswap `auto` |

Confusing these four is the #1 positioning error in this category.
Juggler's spec keeps them separate: switch-profile (one click, new
sessions only), takeover (two clicks), return (one click, suggested)
in [SPEC.md](../../SPEC.md#L284).
Competitors mostly ship only the first two verbs.

### 1.3 Evidence key

- `[repo]` — verified in this repository at the cited `file:line`.
- `[docs]` — competitor README/docs/changelog via web research.
- `[inferred]` — synthesis judgement; flagged so the reader can
  discount it. Star counts are snapshots, never rankings.

---
## 2. Native baselines first: `/usage`, statusline, Codex `/status`

Any third-party quota UI now competes with the vendor's own UI.
This is the bar — and it rose during 2026.

### 2.1 Claude Code: `/usage` + statusline `rate_limits`

- `[docs]` Since Claude Code v2.1.80+, the statusLine command
  receives a `rate_limits` object on stdin (`five_hour` +
  `seven_day`, each `used_percentage` + `resets_at`) — official,
  deterministic, zero token cost; reset epochs match the `/usage`
  TUI exactly. Print-mode `/usage` scraping is dead on v2.1.207
  (insights-only). Source: [sergeiwallace/ai-cli-utils research commit](https://github.com/sergeiwallace/ai-cli-utils/commit/54baf74424d2fcaaaa1b2efb99e1b1d6a05cdde8).
- `[docs]` A dependency-free statusline shows model, context %,
  5h/weekly rate limits, tokens and session cost on one line:
  [savinofiore/claude-usage-statusline](https://github.com/savinofiore/claude-usage-statusline).
  The 5-hour window is rolling (resets 5h after first message);
  the 7-day window is weekly rolling: [AndyJae/claude-code-usage](https://github.com/AndyJae/claude-code-usage).
- `[docs]` `cc-limits` shows the same in realtime: context, 5h,
  7d with time-until-reset: [sslinnn/cc-limits](https://github.com/sslinnn/cc-limits).
  A quota shim can persist the statusline snapshot for other tools:
  [ffffhx/open-token-board commit](https://github.com/ffffhx/open-token-board/commit/b62cce6629faa52e99e9d5572eafa81583426a29).

**Implication for Juggler.** The spec already names the right
source: "install a managed statusline script per profile dir; its
stdin JSON carries `rate_limits.five_hour/seven_day`" as **Measured**
in [SPEC.md](../../SPEC.md#L191).
The domain type exists — `QuotaWindow { used_percentage,
resets_at }` in
[model.rs](../../../crates/juggler-core/src/model.rs#L277) —
and the capability bit exists in
[capabilities.rs](../../../crates/juggler-adapter/src/capabilities.rs#L21).
But grep finds no statusline installer and no producer of a
non-default `UsageSnapshot`: sessions are created with
`UsageSnapshot::default()` in
[commands.rs](../../../crates/juggler-app/src/commands.rs#L706).
`[repo]` The feed is *declared*, not *wired*.

### 2.2 Codex: `/status`, `/wham/usage`, `token_count` snapshots

- `[docs]` Inside Codex CLI, `/status` prints remaining 5-hour
  and weekly quota % with exact reset timestamps (e.g. CLI 0.153.4,
  Oct 2026): [codexusage.dev guide](https://www.codexusage.dev/).
- `[docs]` There is no `codex usage` subcommand; usage lives in
  the TUI status card/status line, polled from `/wham/usage` every
  ~60s, flagged stale after 15 min: [wez/quotas docs](https://github.com/wez/quotas/blob/HEAD/docs-usage/codex.md).
- `[docs]` Every `token_count` event in Codex session logs carries
  `rate_limits` (`primary.used_percent`, `window_minutes: 300`,
  `resets_at`) — parseable without any API call:
  [Medium quota-app writeup](https://medium.com/@vinayanand2/how-much-ai-quota-do-you-have-left-i-built-an-app-for-easy-access-11a53791a064).
  A macOS menu-bar reader exists: [sangimed/codex-menubar](https://github.com/sangimed/codex-menubar).
- `[docs]` There is **no** public stable "remaining %" API; the
  honest options are passive (parse snapshots, classify limit
  errors) rather than reverse-engineering private routes:
  [jimyungsik/omo-account-router design](https://github.com/jimyungsik/omo-account-router/blob/HEAD/docs/usage-remaining-design.md).

**Implication for Juggler.** The spec's Codex stance — token burn
Measured, plan-credit correlation Estimated and labelled — in
[SPEC.md](../../SPEC.md#L198)
is *more* honest than most tools, but it is beatable: parsing the
`token_count` snapshots upgrades Codex quota from Estimated to
Measured for the primary/secondary windows. `[inferred]` That
parse should be scheduled with the Codex adapter, not after it.

### 2.3 Baseline scoreboard (what a user gets with zero extra tools)

| Surface | Claude Code native | Codex native |
|---|---|---|
| Current-window % | `/usage` + statusline | `/status` in TUI |
| Reset time | Epoch + countdown | Exact timestamp |
| Multi-account view | None (one login) | None (one login) |
| Cost in USD | Session cost in statusline | Not subscription % |
| History/trends | None | None |

The gap in the last three rows *is* the third-party market.
Everything else a switcher adds must beat row 1–2 on accuracy.

## 3. Vibe Kanban (Bloop) — the kanban orchestrator that died

### 3.1 What it does

`[docs]` Kanban-over-agents: cards move To Do → In Progress →
Review → Done; moving a card to In Progress auto-creates a git
worktree and starts an agent. Agent-agnostic (Claude Code, Codex,
Gemini CLI, Copilot, Amp, Cursor, OpenCode…), own MCP server,
integrated diff view with inline comments sent back to the agent,
PR creation via `gh`. Repo: [BloopAI/vibe-kanban](https://github.com/BloopAI/vibe-kanban).
Parent review summary is in the full review's Vibe deep-dive.

### 3.2 Multi-account switching mechanics

None native. `[docs]` Vibe is BYO-subscription/keys with no
rotation or per-account dashboard (parent review, Q7). Switching
"accounts" means switching the agent binary on a card — a harness
switch, not a quota switch. There is no equivalent of Juggler's
switch-profile/takeover/return verbs (§1.2).

### 3.3 Quota data source / accuracy / reset handling

None. No 5h/weekly %, no reset countdown, no burn-down. Quota
pain is delegated to the underlying CLIs — exactly the gap the
parent review calls out: orchestrators delegate quota/accounts
to the CLIs (full review Q7). `[inferred]` A Vibe user learns
about the quota wall when a card's agent dies mid-attempt.

### 3.4 Cost analytics

None subscription-oriented. No USD ledger, no per-account cost
rollup. (Cost analytics live in the separate `ccusage` ecosystem,
§6.) Vibe's data model is tasks-in-SQLite, not usage events.

### 3.5 Session features

Strongest-in-class parallel UX: worktree+branch per task-attempt
(per repo in multi-repo), terminal + dev-server + preview browser
per workspace, diff review loop, guarded squash-merge/archive.
Weaknesses (parent review): tasks in SQLite not git; manual
checkouts/external PRs unrecognised; worktree deletion loses the
card link; per-CLI output adapters brittle; no rules/quality gates.

### 3.6 Pricing / OSS / status — the sunset

- Free/self-hosted, Apache-2.0 (some posts wrongly say MIT).
- `[docs]` "Upstream is sunsetting. Bloop, the company behind
  vibe-kanban, shut down on 2026-04-10, and v0.1.44 (2026-04-24)
  is the last release": [docker-vibe-kanban REMOTE_ACCESS](https://github.com/gwwtests/docker-vibe-kanban/blob/HEAD/docs/REMOTE_ACCESS.md).
- `[docs]` Dead-project audit: "Last push 2026-04-24; 0 PRs merged
  in the 90 days since; 151 open PRs stranded": [kayushkin/llm-bridge RELEASE-TARGETS](https://github.com/kayushkin/llm-bridge/blob/HEAD/RELEASE-TARGETS.md).
- `[docs]` Homepage notice "Vibe Kanban is sunsetting… hosted
  offering winding down, maintenance by volunteers": [aq.dev alternatives page](https://aq.dev/alternatives/vibe-kanban/).
  A local-first fork continues it: [mdevel-uy/vibe-kanban](https://github.com/mdevel-uy/vibe-kanban).

### 3.7 Why users love it / leave it

Love: the card=worktree=agent metaphor — the most-copied UX in
the category (parent review). Leave: not by choice — the vendor
died. `[inferred]` The forced migration is the opportunity; see §9
for sizing and what these users actually need (worktrees + diff
review), which is *not* Juggler's current wedge.

### 3.8 vs Juggler (feature-by-feature)

`[repo]` Juggler has no worktree/branch isolation: a session has
only a `cwd` (parent review F1.1). Vibe wins isolation, diff
review, PR flow. Juggler (specced) wins quota + handover lineage:
takeover/return in
[SPEC.md](../../SPEC.md#L284).
Neither ships the other's strength today. Do not pitch Juggler
to Vibe orphans on quota alone — pitch it once worktree-on-launch
exists, or explicitly as the *quota companion* beside their next
orchestrator.

## 4. Crystal (Stravu) → Nimbalyst — the worktree GUI that pivoted

### 4.1 What it does

`[docs]` MIT Electron desktop: parallel Claude Code/Codex
sessions, each in a git worktree; session list + diff/compare
panes; rebase/squash tooling. Repo: [stravu/crystal](https://github.com/stravu/crystal).
Crystal is the cleanest reference for "compare two agents'
approaches side by side" (parent research summaries).

### 4.2 Multi-account switching mechanics

None as a switcher. Crystal isolates by *worktree*, not by
*account*: parallel sessions share the user's single CLI login.
There is no named-profile registry, no per-account quota, no
rotation. (Same delegation pattern as Vibe, §3.2.)

### 4.3 Quota / cost / session features

- Quota: none native (no 5h/weekly, no resets). Cost: none.
- Sessions: worktree manager, terminal-session manager,
  git-diff/status managers, pluggable Claude Code + Codex CLI
  registry — `[docs]` file inventory in [vincenthanxiaodu/oh-my-term research](https://github.com/vincenthanxiaodu/oh-my-term/blob/HEAD/docs/research/remote-agent-products.md).
  The durable pattern is "GUI derived from the CLI's own on-disk
  state" — which Juggler's transcript watcher also follows.

### 4.4 Pricing / OSS / deprecation

- MIT, ~3.1k stars at deprecation. `[docs]` "Crystal (formerly a
  multi-session AI code assistant manager) has been deprecated
  and replaced by **Nimbalyst**. Deprecated: February 2026":
  [stravu/crystal README](https://github.com/stravu/crystal).
  Migration points to [nimbalyst.com/crystal](https://nimbalyst.com/crystal/).
- `[docs]` Analysis of the pivot: "session-management-on-top-of-
  terminals was too thin a wedge" — the team moved to a multi-
  editor AI workspace: [dearken10/baton prior-art](https://github.com/dearken10/baton/blob/HEAD/docs/prior-art.md).
  Corroborated as "deprecated 2026-02 and replaced by Nimbalyst,
  an AI-native workspace": [alp82/curia research](https://github.com/alp82/curia/blob/HEAD/docs/research/agent-face-tools.md).

### 4.5 Why users love it / leave it

Love: polished per-agent workspace UX, side-by-side compare.
Leave: forced — deprecated Feb 2026 (v0.3.5), successor is
commercial Nimbalyst. Parent review: "beloved parallel-session
UX, unsustained as a free standalone" (full review).

### 4.6 vs Juggler + the cautionary lesson

`[repo]` Juggler's supervisor + embedded-terminal design is the
same wedge Crystal walked away from. The difference must be
*supervision value* (stuck/quota/lineage), not terminal hosting:
Juggler models `Stuck`/`WaitingUser`/`QuotaBlocked` in
[model.rs](../../../crates/juggler-core/src/model.rs#L188)
but — per parent review F2.5 — never emits the last three.
`[inferred]` If those statuses stay unemitted, Juggler repeats
Crystal's mistake with better types. Crystal's MIT code remains
a legitimate reference for diff/compare UX; its users need a
free worktree GUI *now*, which Juggler is not yet (§3.8).

## 5. cc-switch (farion1231) — the config-switching incumbent

### 5.1 What it does

`[docs]` "A cross-platform desktop All-in-One assistant for
Claude Code, Codex, OpenCode, OpenClaw, Grok Build & Hermes
Agent" — switch API providers in one click and manage MCP,
Skills and Prompts in one place, "no more hand-editing JSON /
TOML / YAML config files": [farion1231/cc-switch](https://github.com/farion1231/cc-switch).
Official site: ccswitch.io (same repo README).

### 5.2 Multi-account / provider switching mechanics

- **Config switch, not session transplant** (§1.2, verb 1): one
  click rewrites the live provider config; tray quick-switch;
  Claude Code needs no restart (parent review). Universal
  providers sync one config to Claude Code / Codex / Gemini CLI:
  [zleap-admin/302-cc-switch feature summary](https://github.com/zleap-admin/302-cc-switch).
- **Projects (v3.17.0, 2026-07-13):** save current provider +
  MCP + Skills + prompt files as a named snapshot; switch the
  whole set in one click from title bar or tray; leaving state
  auto-saves back: [cc-switch v3.17.0 changelog](https://github.com/farion1231/cc-switch-website/blob/HEAD/public/docs/changelog/en/3.17.0.md).
  Projects are now global shared entities with per-tool scoped
  switching: [cc-switch profiles refactor commit](https://github.com/farion1231/cc-switch/commit/dbb5999d1ec71fb047cc62e4f092dffaec123325).
- **MCP sync:** upserting an enabled MCP server auto-syncs to
  the app config: [cc-switch MCP auto-sync commit](https://github.com/farion1231/cc-switch/commit/e77eab211605442328fc2b3e04f8662682af2742);
  sync is per-server fault-isolated: [cc-switch MCP sync fix](https://github.com/farion1231/cc-switch/commit/81c0ff252b830584d086f0e327ab1edf93afb507).
  Provider switches sync MCP for all apps: [cc-switch MCP-on-switch commit](https://github.com/farion1231/cc-switch/commit/a00eb764f7da12ee9dfe13ff920c098fb10d94bd).

### 5.3 Quota data source / accuracy / reset handling

- `[docs]` Inline 5h/weekly quota/balance (incl. Copilot/Codex
  OAuth) in the tray row `name · mode · provider · quota`
  (parent review). Quota is provider-balance oriented: API-key
  and coding-plan balances, not only subscription windows.
- `[docs]` Coding-plan quota adapters exist per provider
  (kimi/zhipu/minimax/volcengine); a missing adapter (iFlytek)
  means "cannot query remaining quota like users of other
  providers": [cc-switch issue #4840](https://github.com/farion1231/cc-switch/issues/4840).
  Accuracy is therefore *per-provider uneven* — a structural
  weakness Juggler's Measured/Estimated labelling can exploit.
- `[docs]` Statusline companions read quota however you switch
  ("Works however you switch providers — CC Switch, manual edits,
  or any other tool"): [baigong-ai/cc-planbar](https://github.com/baigong-ai/cc-planbar).

### 5.4 Cost analytics

- `[docs]` Usage statistics: spending/requests/tokens dashboards
  per model; **models.dev automatic pricing sync** (opt-in) plus a
  human-editable `~/.cc-switch/model-pricing.json` (v3.19.0):
  [cc-switch v3.19.0 release notes](https://github.com/farion1231/cc-switch/blob/HEAD/docs/release-notes/v3.19.0-en.md).
  Grok OAuth-mode usage import closes a previously invisible gap
  (same notes). Storage is SQLite SSOT at
  `~/.cc-switch/cc-switch.db` with auto-rotating backups:
  [theresanaiforthat cc-switch profile](https://theresanaiforthat.com/company/farion1231/repository/cc-switch/).

### 5.5 Session features / pricing / OSS

- Sessions: none supervised. cc-switch configures *future* CLI
  runs; it does not watch live sessions, detect stuck, or keep
  lineage. That absence is Juggler's opening (§8).
- Free, MIT (fork READMEs confirm "fork of cc-switch (MIT)"):
  [wanjiaruan/302-cc-switch](https://github.com/wanjiaruan/302-cc-switch).
  Very large star count in search snapshots (~140k string in repo
  metadata snippets — treat as unverified magnitude, Appendix).

### 5.6 Why users love it / leave it

Love: one-click switching without restart, tray-at-a-glance,
no more JSON editing, broad tool coverage (7–10 CLIs).
Leave (`[inferred]` from the adapter-gap evidence): uneven
quota accuracy per provider, config-switch surprises (state
auto-save on project switch), and no help once sessions are
*running* — the tool's job ends at launch.

### 5.7 vs Juggler

`[repo]` Juggler's tray is specced quota-only
([SPEC.md](../../SPEC.md#L352))
and absent in `ui/src` (parent review F7.2); the Usage screen is
`ComingSoon` in
[App.tsx](../../../ui/src/App.tsx#L339).
cc-switch therefore beats Juggler today on every switcher
expectation: tray switch, quota glance, MCP/skills sync, pricing
sync. Juggler's counter is supervision + handover + honesty
labels — none of which cc-switch attempts. See §8, S1–S5.

## 6. ccusage (ryoppippi) — the cost-accounting standard

### 6.1 What it does

`[docs]` CLI reading `~/.claude/projects` JSONL (+ Codex/
OpenCode/Amp sources), producing daily/monthly/session/5h-block
token + API-pricing cost reports (parent review). Repo:
[ryoppippi/ccusage](https://github.com/ryoppippi/ccusage).
It answers "what did I burn?" — never "what do I have left?".

### 6.2 Switching mechanics

None. ccusage is read-only accounting; it does not switch,
rotate, or launch. Its role in the stack is the *ledger* that
switchers lack — `cswap` users pair it for spend (parent review).

### 6.3 Quota data source / accuracy / reset handling

- Source: local transcripts only. Cost is **Computed** (§1.1):
  tokens × price table. `[docs]` Pricing can run offline with an
  embedded LiteLLM snapshot; Nix builds embed the locked pricing
  file so sandboxed builds fetch nothing: [alexejboiko/ccusage](https://github.com/alexejboiko/ccusage).
  Forks add `--share` runs with offline pricing by default:
  [leoisadev1/ccusage-plus](https://github.com/leoisadev1/ccusage-plus).
- Known accuracy trap: `[docs]` "Claude Code's `.jsonl`
  transcripts contain no `costUSD` field for subscription usage,
  so cost is *always* computed from a price table fetched over
  the network" — plus two more silent-error modes documented in
  [vibetec-eu/ccdash](https://github.com/vibetec-eu/ccdash).
  Per-prompt (not just daily) breakdowns exist downstream:
  [haruaki-nayuta/ccmeter](https://github.com/haruaki-nayuta/ccmeter).
- Resets: the 5h-block report aligns to Claude's rolling window,
  but it reports *burn inside blocks*, not vendor remaining %.
  No reset countdown, no headroom — by design.

### 6.4 Cost analytics (its whole product)

Daily/monthly/blocks/session reports, per-model splits,
multi-source (Claude/Codex/OpenCode/Amp), statusline mode.
`[docs]` Complementary-tools framing: ccusage does "macro usage
aggregates… plus a statusline" while deep per-session audit
lives elsewhere: [axisrow/claude-devtools README](https://github.com/axisrow/claude-devtools/blob/HEAD/README.md).

### 6.5 Sessions / pricing / OSS / love-leave

- Sessions: historical only (which session burned what). No live
  supervision, no resume, no lineage.
- Free OSS CLI (~13–17k stars, parent review snapshot).
- Love: trustworthy-enough money numbers from data already on
  disk; scriptable; offline mode. Leave: numbers silently wrong
  when pricing fetch fails or transcripts lack cost fields
  (ccdash evidence above) — users discover this only by auditing.

### 6.6 vs Juggler

`[repo]` Juggler already parses the same JSONL and already has
`TokenCounts` per session in
[model.rs](../../../crates/juggler-core/src/model.rs#L261),
but models tokens/credits only — no pricing table, no USD
(parent review F1.2). The spec's usage engine promises
per-session/account/window aggregates + burn-down in
[SPEC.md](../../SPEC.md#L254).
`[inferred]` Shipping ccusage-parity (daily/monthly/blocks +
offline price snapshot + CSV) is the cheapest credibility win
available: the input pipeline exists; only pricing + rollups
are missing. Label every USD figure Computed, never Measured.

## 7. claude-swap (`cswap`) + `aisw` — true account juggling

### 7.1 claude-swap: what it does

`[docs]` "Switch between multiple Claude Code accounts, with
automatic rate-limit rotation, usage dashboard, and parallel
sessions": [realiti4/claude-swap](https://github.com/realiti4/claude-swap).
CLI (`cswap`) + TUI + optional menubar. Claude-only. This is
Juggler's closest incumbent: the only tool whose *headline* is
juggling (§1.2, verbs 2 + 4).

### 7.2 Switching mechanics (the day-1 bar)

- `[docs]` `cswap switch` rotates to the next account;
  `cswap switch 2` / `switch user@example.com` / alias targets a
  specific one; leaving-account token is stored (same README:
  [realiti4/claude-swap](https://github.com/realiti4/claude-swap)).
- `[docs]` **Auto-rotation:** "When the active account's 5-hour
  or 7-day window reaches the threshold (default 90%), it
  switches to the account with the most quota left — before you
  hit the limit, and safe to run while Claude Code is working":
  `cswap auto` (polls every 60s), `--threshold 80`, `--once`,
  `--json`, `--dry-run`: [cswap v0.16.0 release](https://github.com/realiti4/claude-swap/releases/tag/v0.16.0)
  and [PR #81](https://github.com/realiti4/claude-swap/pull/81).
- `[docs]` Usage-aware manual switch: `--switch --best` picks
  most remaining 5h/7d headroom and "never moves you onto an
  account worse than your current one": [PR #59](https://github.com/realiti4/claude-swap/pull/59).
  Menubar shows every account's 5h/7d/spend and switches by
  click (specific/rotate/best/next-available), sharing the
  `autoswitch.*` settings with the CLI (same README).
- `[docs]` Directory binding: `cswap map` binds a directory to
  an account so bare `cswap run` there launches that account:
  [hainnt/c-hottag comparison](https://github.com/hainnt/c-hottag/blob/HEAD/docs/comparison.md).
  Desktop wrapper adds aliases, disable/enable (hold out of
  rotation), threshold/strategy editing, switch notifications:
  [theskinnyrat/cswap-desktop](https://github.com/theskinnyrat/cswap-desktop).

### 7.3 Quota source / accuracy / reset handling

- `[docs]` `cswap list` is the dashboard: "every account's
  5-hour and 7-day usage and reset times at a glance" (README,
  [realiti4/claude-swap](https://github.com/realiti4/claude-swap)).
  Source is per-account credential-backed usage fetch (Measured-
  class, §1.1) — which creates its known failure mode:
- `[docs]` Token-family desync: once `cswap run` bootstraps a
  profile, Claude rotates the token family *inside* the profile;
  the backup's refresh token 401s forever and "usage for that
  account then freezes silently at the last pre-session
  measurement": [PR #97](https://github.com/realiti4/claude-swap/pull/97).
  Active-account refresh has the mirror bug: [PR #70](https://github.com/realiti4/claude-swap/pull/70).
  `[inferred]` Silent staleness is the switcher category's
  original sin — Juggler's freshness/stale-flag design must beat
  this explicitly (Codex's 15-min stale flag, §2.2, is the model).
- Resets: shown per account; auto-switch acts *before* the wall.
  No burn-down forecast ("exhausts in ~40m") found in docs.

### 7.4 Cost / sessions / pricing / love-leave

- Cost: spend column in menubar; deep cost via `ccusage` pairing
  (§6.2). No built-in USD ledger comparable to cc-switch (§5.4).
- Sessions: parallel sessions supported; maintained-fork activity
  (`aswap` = upstream + unmerged fixes) signals both love and
  upstream-fix latency: [blackhole1/aswap](https://github.com/blackhole1/aswap/blob/HEAD/README.md).
- Free OSS. Love: set-and-forget rotation, best-account picking,
  per-directory accounts. Leave: Claude-only, silent-stale usage
  bugs, rotation is *automatic* (see §7.6 compliance contrast).

### 7.5 `aisw` (burakdede): named profiles across CLIs

`[docs]` "Switch between multiple Claude Code, Codex CLI,
Antigravity and Gemini CLI accounts in one command. Named
profile manager for AI coding agents": [burakdede/aisw](https://github.com/burakdede/aisw).
Mechanics: `aisw use claude acct`, `aisw use --all --profile
acct`; "After `aisw use …`, start a fresh agent process… you
should not rely on reusing an already running or resumable
session from the previous account" (same README). Quota: none
shown in README evidence (negative evidence only). Cost: none.
Sessions: fresh-process switching only — no transplant, no
lineage. Love: one-command, multi-CLI, explicit. Leave: restart-
to-switch friction that cc-switch (no-restart) already solved.

### 7.6 vs Juggler (feature-by-feature)

`[repo]` Juggler's invariant forbids `cswap auto`'s headline:
automation is passive (monitor/alert/suggest); every state-
changing action is an explicit click, spec §7 in
[SPEC.md](../../SPEC.md#L261).
The compliant answer is *suggested* rotation ("A at 92% — switch
new sessions to B? [Switch]"), never auto-rotation. But the
*information* `cswap auto` acts on — per-account 5h/7d + resets +
best-account ranking — is table stakes (§8, S2/S4). Juggler's
handover entry point exists in
[commands.rs](../../../crates/juggler-app/src/commands.rs#L1070)
(`session_handover`), yet advertised `TRANSPLANT` in
[claude_code.rs](../../../crates/juggler-adapters/src/claude_code.rs#L111)
outruns the proven single-JSONL copy (parent review F3.3).
Niche map: choose **cswap** for one-active-account auto-rotation,
**aisw** for multi-CLI named profiles, per [nemo-illusionist comparison](https://github.com/nemo-illusionist/claude-code-account-switcher/blob/HEAD/docs/comparison.md) —
Juggler must win the third cell: *parallel supervised* juggling.

## 8. Day-1 table stakes switcher users expect (vs Juggler today)

| # | Expectation (source) | Juggler today `[repo]` |
|---|---|---|
| S1 | One-click switch, no restart (cc-switch §5.2) | Profile launch only; no tray switch |
| S2 | Every account 5h/7d % + reset at a glance (`cswap list` §7.3) | `percent={0}` rings — missing |
| S3 | Quota as truthful as `/usage` (§2.1) | Feed declared, not wired (§2.1) |
| S4 | Best-account ranking + threshold warning (`--best`, 90% §7.2) | None; suggest-only version specced |
| S5 | Named profiles + per-dir affinity (`aisw use`, `cswap map`) | Profiles exist; no dir affinity |
| S6 | USD ledger, offline pricing, CSV (ccusage §6, cc-switch §5.4) | Tokens only, no USD (§6.6) |
| S7 | MCP/skills/prompt sync across accounts (cc-switch §5.2) | Not built (parent F1.9) |
| S8 | Freshness you can see; stale never silent (§7.3, §2.2) | `refresh()` swallows errors (parent F2.1) |
| S9 | Worktree isolation + diff review (Vibe/Crystal §§3–4) | No worktree (parent F1.1) |
| S10 | Live-session help after launch (nobody ships this) | Modelled, unemitted statuses (§4.6) |

S1–S6 are *switcher* table stakes: without them a `cswap`/
cc-switch user churns in day 1. S9 is the *orchestrator* stake
for Vibe/Crystal orphans. S10 is the wedge: nobody ships it, and
it is the only row where Juggler can be first. Sequence: S2+S3
(wire the statusline feed into `QuotaWindow`), S8 (visible
freshness), S4-as-suggestion, S6, then S1/S5/S7 tray parity;
S9 before marketing to orphans; S10 as the headline.

## 9. Sunset / migration opportunity — sized honestly

- Vibe: ~26.9k stars pre-sunset `[docs]` [codecoradev issue #312](https://github.com/codecoradev/uteke/issues/312);
  151 stranded PRs (§3.6). Crystal: ~3.1k stars (§4.4). These are
  *audience* proxies, not movable users — no download/active-user
  figure was verified (Appendix). `[inferred]` Realistic capture
  needs (a) import of projects/worktrees, (b) S9, (c) a comparison
  page — in that order. Quota messaging alone captures ~none.
- The switcher incumbents are *not* sunsetting: cc-switch and
  `cswap` are actively releasing (v3.19.0, v0.16.0 above). There
  is no orphan pool there — only head-to-head wins on S8/S10.
- Timing: Vibe's CDN-dependent `npx` install "will break when
  the CDN goes away" (§3.6) — the migration window closes when
  orphans land on Conductor/T3/Superset-style successors, e.g.
  [Superset vs Vibe comparison](https://github.com/kogan/superset/blob/HEAD/apps/marketing/content/compare/superset-vs-vibe-kanban.mdx).

## 10. Bottom line for the roadmap

1. Stop advertising `USAGE_QUOTA_FEED|TRANSPLANT` until wired/
   proven (§2.1, §7.6) — switcher users audit this first.
2. Ship S2/S3/S8 as one slice: statusline feed → `QuotaWindow`
   → labelled rings with reset countdowns + stale flags.
3. Answer `cswap auto` with suggested rotation + reset-queue
   (one click), preserving the passive invariant (§7.6).
4. Add ccusage-parity cost (Computed, offline prices, CSV).
5. Earn Vibe/Crystal orphans with worktrees + import, not copy.

## Appendix — unresolved evidence (do not quote as fact)

- No live install/run of any competitor; CLI behaviours are
  `[docs]` from README/changelog/PR evidence, not execution.
- Star counts are search snapshots (cc-switch ~140k string
  unverified; Vibe 26.9k via secondary issue; Crystal 3.1k).
- `aisw` quota absence and Vibe/Crystal quota absence are
  negative evidence (no feature found), not vendor denials.
- Vibe active-user/download counts, Nimbalyst pricing, and
  `cswap` usage-API internals were not primary-verified.
- Juggler line refs verified 2026-10-13 on `main`; UI absence
  claims rely on parent-review grep + §5.7 spot checks.







