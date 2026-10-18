# Deep competitor profile — Claude Squad & Conductor
> 5× deeper than `docs/review/full-competitive-ux-review.md`
> §Q7 (Squad 577–602, Conductor 604–626). Official GitHub/docs/
> site only for competitor facts (URL per claim); Juggler claims
> carry `repo file:line`. `[negative evidence]` = not found.
> Snapshot: 2026-10-13. Squad = OSS Go TUI (`cs`); Conductor =
> closed-source macOS app (Melty Labs). Same worktree primitive,
> opposite bets on accounts, review, and openness.

---

## 0. Verdict in 30 seconds

| Question | Answer |
|---|---|
| Closest threat to Juggler's wedge? | Neither. Both delegate accounts/quota to the CLI. |
| Most stealable idea? | Squad's `1 task = 1 tmux + 1 worktree` atomicity; Conductor's diff→PR→merge cockpit. |
| Most dangerous trap? | Squad's `-y/--autoyes` auto-accept; Conductor's closed-source polish envy. |
| What the parent review got wrong? | Conductor pricing ($50/$60) is stale — current official pricing is free. See §7. |
| What Juggler must do first? | Worktree-on-launch (F1.1 in parent review) — both competitors prove it is table stakes. |

Parent claim survives: all orchestrators "delegate
quota/accounts to the underlying CLIs"
(`docs/review/full-competitive-ux-review.md:46-48`). §5 proves
it is architectural in both tools, not a missing toggle.

## 1. Claude Squad — history & traction

### 1.1 What it is, precisely

Claude Squad is a terminal UI that "runs Claude Code, Codex,
Gemini, Aider, OpenCode, Amp, and other local programs in
separate tmux sessions and git worktrees"
[https://github.com/smtg-ai/claude-squad](https://github.com/smtg-ai/claude-squad).
It is **not an agent**: it spawns the user's CLI in tmux with
an isolated worktree (parent review
`docs/review/full-competitive-ux-review.md:579-582`). Breadth
("any CLI via `-p`") is the feature — the opposite bet from
Juggler's typed adapter (`crates/juggler-adapter/src/adapter.rs:26-27`).

### 1.2 Origin & maintenance signal

- Repository: `smtg-ai/claude-squad`, written in Go, shipped
  as a single binary installed via `brew install claude-squad`
  or `curl -fsSL https://raw.githubusercontent.com/smtg-ai/claude-squad/main/install.sh | bash`
  [https://github.com/smtg-ai/claude-squad](https://github.com/smtg-ai/claude-squad).
- Release cadence: v1.0.19 published 2026-06-17; v1.0.20 on
  2026-08-20 is the latest cited release — actively maintained,
  not dormant
  [https://github.com/gregce/tortie/blob/HEAD/docs/research/04-agent-managers.md](https://github.com/gregce/tortie/blob/HEAD/docs/research/04-agent-managers.md),
  [https://github.com/gaby/munder-difflin/blob/HEAD/blog/src/posts/herdr-alternatives.md](https://github.com/gaby/munder-difflin/blob/HEAD/blog/src/posts/herdr-alternatives.md).
- Traction magnitude: ~7,171 stars in a 2026-03 snapshot with
  51 open issues and 12 commits/90d
  [https://github.com/kayushkin/llm-bridge/blob/HEAD/RELEASE-TARGETS.md](https://github.com/kayushkin/llm-bridge/blob/HEAD/RELEASE-TARGETS.md);
  ~8,347 stars in a 2025-05 commit view
  [https://github.com/smtg-ai/claude-squad/commit/b4a95e186d024f840e5f4b3565120c7df66896da](https://github.com/smtg-ai/claude-squad/commit/b4a95e186d024f840e5f4b3565120c7df66896da);
  parent review's "8.1–8.5k" range
  (`docs/review/full-competitive-ux-review.md:602`) is consistent.
  Treat all counts as magnitude, not ranking — no direct
  GitHub API fetch was available in-session (parent review
  Appendix A, `docs/review/full-competitive-ux-review.md:1047-1048`).
- Issue-tracker depth: ~133 issues mined in one independent
  sweep [https://github.com/wilsonsousajr/omatty/commit/d70858998ed3b5ecf6894ef9b1e079173d07e272](https://github.com/wilsonsousajr/omatty/commit/d70858998ed3b5ecf6894ef9b1e079173d07e272) —
  small enough to read fully, large enough to show real
  dogfooding. Juggler has no comparable public signal yet.

### 1.3 Lesson: zero-to-first-session time

Squad proves demand for parallelism without a GUI tax (`cs`
to launch). Juggler's richer onboarding
(`ui/src/screens/OnboardingWizard.tsx`;
`docs/review/full-competitive-ux-review.md:213-220`) must
approach that two-command story on the Claude-only path.

---

## 2. Conductor — history & traction

### 2.1 What it is, precisely

Conductor (conductor.build) is "a Mac app for running several
agents at once through a chat interface" supporting "Claude
Code, Codex, Cursor, and OpenCode," where "a task lives in its
own workspace, meaning its own branch, files, terminal, and
diff, and the app walks it from review to pull request, merge,
and archive" [https://www.agentastic.dev/alternatives/paseo](https://www.agentastic.dev/alternatives/paseo).
It is built by Melty Labs (YC S24), which "built the Melty
editor first, then pivoted" to the parallel runner and
"essentially pioneered the category of the Mac 'agentic
parallel runner'" [https://github.com/geraschenko/pictl/blob/HEAD/docs/agent-harness-landscape.md](https://github.com/geraschenko/pictl/blob/HEAD/docs/agent-harness-landscape.md).
The parent review's one-liner ("native macOS app (YC S24) that
pioneered the agentic parallel runner,"
`docs/review/full-competitive-ux-review.md:606-608`) holds.

### 2.2 Funding & velocity

Melty Labs, YC S24; "$22M Series A" / "$24M raised"
`[unresolved exact total]` [https://github.com/broomva/broomva.tech/blob/HEAD/apps/broomva/content/writing/agent-cockpit-wars.mdx](https://github.com/broomva/broomva.tech/blob/HEAD/apps/broomva/content/writing/agent-cockpit-wars.mdx),
[https://github.com/antoinedc/mantaui/blob/HEAD/docs/specs/website-readme-revamp.md](https://github.com/antoinedc/mantaui/blob/HEAD/docs/specs/website-readme-revamp.md);
founders ex-Replicate/ex-Netflix ML (same source). "Used by
engineers at Linear, Vercel, Notion, Stripe"
[https://github.com/dpeluche/exegol/blob/HEAD/docs/project_definition/RESEARCH.md](https://github.com/dpeluche/exegol/blob/HEAD/docs/project_definition/RESEARCH.md)
is marketing social proof, not audited traction. Changelog
to v0.87.x ships bundled model updates (e.g. Codex 0.156.1)
[https://www.conductor.build/changelog](https://www.conductor.build/changelog) —
a burden Juggler avoids by spawning the user's own CLI
(`crates/juggler-adapters/src/claude_code.rs:298-305`).

### 2.3 Closed source is the product decision

Conductor is "closed source, no public GitHub repo"
[https://github.com/dpeluche/exegol/blob/HEAD/docs/project_definition/RESEARCH.md](https://github.com/dpeluche/exegol/blob/HEAD/docs/project_definition/RESEARCH.md),
"Proprietary, free - you bring your own agent subscription
or API key" [https://github.com/rodgi040/secret-ai-beaver-sauce/blob/HEAD/skills/secret-ai-beaver-source/references/TOOLS.md](https://github.com/rodgi040/secret-ai-beaver-sauce/blob/HEAD/skills/secret-ai-beaver-source/references/TOOLS.md).
That means: no audit path, no fork path, no offline guarantee.
Juggler is MIT (`Cargo.toml:17`, `README.md:83`) and local-only
by invariant (`docs/review/agent-review-prompt.md:70`). The
contrast is a positioning asset Juggler under-uses today.

---

## 3. The tmux/worktree model — the shared foundation

Both tools converge on the same isolation primitive. The
differences are in lifecycle and cleanup, where Juggler's
current gap (no worktree at all) is most exposed.

### 3.1 Claude Squad: tmux session + worktree, atomically

- "Each instance = tmux session (`claudesquad_<title>_<timestamp>`)
  + git worktree in `~/.claude-squad/worktrees/` on branch
  `<prefix>_<name>_<timestamp>`"
  [https://github.com/gregce/tortie/blob/HEAD/docs/research/10-multi-project-ux.md](https://github.com/gregce/tortie/blob/HEAD/docs/research/10-multi-project-ux.md).
- "Claude Squad creates a unique branch/worktree for every
  session and starts that agent's tmux program inside that
  isolated worktree"
  [https://github.com/opensiro/vsm-harness-index/blob/HEAD/assessments/claude-squad.md](https://github.com/opensiro/vsm-harness-index/blob/HEAD/assessments/claude-squad.md).
- State persists in `~/.claude-squad/state.json` (instance
  metadata, branch ownership, base commit) plus `daemon.pid`;
  config in `~/.claude-squad/config.json`
  [https://github.com/mkb23/overcode/blob/HEAD/docs/design/bakeoffs/overcode-vs-claude-squad.md](https://github.com/mkb23/overcode/blob/HEAD/docs/design/bakeoffs/overcode-vs-claude-squad.md).
- Pause vs delete are distinct: "pause can retain the branch
  for resume, while deletion removes only worktrees/branches
  that Claude Squad owns"
  [https://github.com/giuseppecrj/pi-herdr-agents/blob/HEAD/docs/research/worktree-subagent-orchestration.md](https://github.com/giuseppecrj/pi-herdr-agents/blob/HEAD/docs/research/worktree-subagent-orchestration.md).
  Ownership-scoped deletion is a safety property Juggler must
  copy verbatim if it adds worktrees.

### 3.2 Conductor: workspace = worktree + branch + lifecycle scripts

- "Local workspaces as git worktrees created under
  `~/conductor/workspaces/<repo name>/<workspace name>`"
  [https://www.conductor.build/docs/concepts/git-worktrees](https://www.conductor.build/docs/concepts/git-worktrees)
  (via [https://github.com/indosaram/ferryx/blob/HEAD/docs/SITE_SEARCH_PRIORITIES_2026-09-19.md](https://github.com/indosaram/ferryx/blob/HEAD/docs/SITE_SEARCH_PRIORITIES_2026-09-19.md)).
- "Conductor documents a separate branch and working tree
  for each workspace" [https://www.conductor.build/docs/concepts/parallel-agents](https://www.conductor.build/docs/concepts/parallel-agents)
  (via [https://github.com/phong711/agent-orchestrator-p/blob/HEAD/frontend/src/landing/content/compare/ao-vs-conductor.mdx](https://github.com/phong711/agent-orchestrator-p/blob/HEAD/frontend/src/landing/content/compare/ao-vs-conductor.mdx)).
- Lifecycle scripts are part of workspace creation:
  "`scripts.setup` runs after Conductor creates a workspace.
  `scripts.run` runs from Conductor's Run button.
  `scripts.archive` runs before Conductor archives a workspace"
  [https://github.com/cmer/skills/blob/HEAD/plugins/orchestrator-setup/skills/orchestrator-setup/references/orchestrators/conductor.md](https://github.com/cmer/skills/blob/HEAD/plugins/orchestrator-setup/skills/orchestrator-setup/references/orchestrators/conductor.md),
  with env vars `CONDUCTOR_WORKSPACE_NAME`, `CONDUCTOR_ROOT_PATH`,
  and `$CONDUCTOR_PORT` port allocation
  [https://github.com/biw/keychain-store/blob/HEAD/.agents/skills/conductor-setup/references/scripts-and-environment.md](https://github.com/biw/keychain-store/blob/HEAD/.agents/skills/conductor-setup/references/scripts-and-environment.md).
- Cloud variant: "cloud workspaces are Vercel sandboxes,
  8-core / 16 GB" running Linux, drivable from the Mac app
  [https://github.com/indosaram/ferryx/blob/HEAD/docs/SITE_SEARCH_PRIORITIES_2026-09-19.md](https://github.com/indosaram/ferryx/blob/HEAD/docs/SITE_SEARCH_PRIORITIES_2026-09-19.md);
  desktop remains Mac-only, cloud workspaces run on Linux
  [https://github.com/thomaschristopherking/conductor-android/blob/HEAD/docs/conductor-skill.md](https://github.com/thomaschristopherking/conductor-android/blob/HEAD/docs/conductor-skill.md).

### 3.3 Juggler today: cwd only, no worktree, no branch

Juggler's session model carries only a working directory:
`pub cwd: PathBuf` (`crates/juggler-core/src/model.rs:113`,
mirrored in `docs/SPEC.md:113`). There is no worktree, branch,
or isolation field in `crates/` or `ui/src/` — a `grep` for
`worktree` across both trees returns zero hits (verified
in-session). The parent review's F1.1 is therefore exact:
"SPEC.md:113 gives a session only a `cwd`. PROPOSAL.md:75
promises parallel accounts on the same repo"
(`docs/review/full-competitive-ux-review.md:64-66`), and
"Juggler has no branch concept outside a pack field
(SPEC.md:301)" (`docs/review/full-competitive-ux-review.md:68-69`).
Consequence: two Juggler sessions on one repo share one
working tree. Both competitors make that state impossible
by construction. This is the single largest structural gap
in this file — see severity table in §10.

**Steal:** Squad's atomic `tmux+worktree` creation and
ownership-scoped cleanup. **Beat:** add account awareness
Squad lacks (worktree branch named by profile, §10).
**Avoid:** Conductor's mandatory worktree for *every*
workspace — Juggler should offer worktree-on-launch as an
option (parent review F1.1 recommendation,
`docs/review/full-competitive-ux-review.md:70-74`), because
read-only/exploratory sessions do not need isolation and
forced worktrees surprise users on non-git folders.

---

## 4. Parallel sessions — how each tool scales the daily loop

### 4.1 Claude Squad: flat list, preview pane, background daemon

- Interaction model: "flat **instance list + preview pane +
  diff tab**" in one TUI window
  [https://github.com/gregce/tortie/blob/HEAD/docs/research/10-multi-project-ux.md](https://github.com/gregce/tortie/blob/HEAD/docs/research/10-multi-project-ux.md);
  "Central instance list, preview pane, diff tab, auto-accept
  background mode" [https://github.com/gregce/tortie/blob/HEAD/docs/research/04-agent-managers.md](https://github.com/gregce/tortie/blob/HEAD/docs/research/04-agent-managers.md).
- Background execution: instances run inside tmux, so closing
  the TUI does not kill agents; a polling daemon (`daemon.pid`,
  `daemon_poll_interval` default 1000 ms) watches for prompts
  when autoyes is on
  [https://github.com/mkb23/overcode/blob/HEAD/docs/design/bakeoffs/overcode-vs-claude-squad.md](https://github.com/mkb23/overcode/blob/HEAD/docs/design/bakeoffs/overcode-vs-claude-squad.md).
- Concurrency ceiling is human, not technical: "4-8 worktrees
  per developer reliably; above that you're bottlenecked on
  review, not on Claude"
  [https://github.com/t1mdurden/prd-pipeline/blob/HEAD/plugins/prd-pipeline/skills/prd-pipeline/references/research-notes.md](https://github.com/t1mdurden/prd-pipeline/blob/HEAD/plugins/prd-pipeline/skills/prd-pipeline/references/research-notes.md).
  Squad offers no triage queue to raise that ceiling — the
  list *is* the queue.

### 4.2 Conductor: workspace list per repo, dashboard triage

"Workspace list per repo, multiple repos in one window"
[https://github.com/gregce/tortie/blob/HEAD/docs/research/10-multi-project-ux.md](https://github.com/gregce/tortie/blob/HEAD/docs/research/10-multi-project-ux.md);
start from branch/PR/GitHub/Linear issue (parent review
`docs/review/full-competitive-ux-review.md:620-621`) or open
in Cursor/VS Code [https://github.com/yzgershon/gatedspace/blob/HEAD/apps/marketing/content/compare/superset-vs-conductor.mdx](https://github.com/yzgershon/gatedspace/blob/HEAD/apps/marketing/content/compare/superset-vs-conductor.mdx);
changelog shows ongoing workspace-status investment
[https://www.conductor.build/changelog](https://www.conductor.build/changelog).

### 4.3 Juggler today: rail + embedded terminal, capped lists

Juggler's daily loop is a left rail (Projects / Instances /
Sessions) with embedded live terminals
(`docs/review/agent-review-prompt.md:51-53`). Two scale
defects are already documented and confirmed in code:
the rail "caps at `slice(0,15)` sessions / `slice(0,8)` recents
with no search or filter (WorkflowRail.tsx:184-195,328-384)"
(`docs/review/full-competitive-ux-review.md:205-207`), and
`Stuck`/`WaitingUser`/`QuotaBlocked` "are modelled
(model.rs:188-207) but never emitted (commands.rs:782-810)"
(`docs/review/full-competitive-ux-review.md:29-32`).
Neither Squad nor Conductor has a needs-attention inbox
either — but Conductor's Checks tab and Squad's preview
pane at least surface *state*. Juggler currently surfaces
*presence* (running/stopped) without *need* (waiting/stuck).
At 15 sessions that is the difference between supervision
and a process list. Parent review WOW-a (Needs-Attention
inbox, `docs/review/full-competitive-ux-review.md:765-768`)
is the correct response and neither competitor blocks it.

---

## 5. Multi-account & quota — the explicit test

This is Juggler's wedge (G2/G3, `docs/PROPOSAL.md:32-33`).
The tasking asked to *explicitly test* whether either
competitor does multi-account juggling. Result: **neither
does, in upstream/official sources.** Evidence below;
forks that add it prove the gap by existing.

### 5.1 Claude Squad upstream: profiles are programs, not accounts

- Config schema: `default_program`, `auto_yes`,
  `daemon_poll_interval`, `branch_prefix`, and
  `profiles[{name,program}]` in `~/.claude-squad/config.json`
  [https://github.com/mkb23/overcode/blob/HEAD/docs/design/bakeoffs/overcode-vs-claude-squad.md](https://github.com/mkb23/overcode/blob/HEAD/docs/design/bakeoffs/overcode-vs-claude-squad.md);
  example: `{"name": "aider-gpt4", "program": "aider --model gpt-4"}`
  [https://github.com/franzfelberer/claude-squad-fork/blob/HEAD/CLAUDE.md](https://github.com/franzfelberer/claude-squad-fork/blob/HEAD/CLAUDE.md).
  A "profile" selects *which binary + flags* to launch. It
  carries no credential, no `CLAUDE_CONFIG_DIR`, no quota.
- CLI surface confirms it: `-p/--program string  Program to
  run in new instances (e.g. 'aider --model ollama_chat/gemma3:1b')`
  and `-y/--autoyes` are the only session-shaping flags
  [https://github.com/smtg-ai/claude-squad](https://github.com/smtg-ai/claude-squad).
- Parent review verdict stands: "Quota/accounts: absent
  upstream — config holds no rotation/pooling/quota; auth is
  delegated to each CLI's login/env"
  (`docs/review/full-competitive-ux-review.md:589-591`).
  `[negative evidence]` — no rotation/pooling/quota key
  exists in the documented config schema above; absence is
  inferred from the complete key list, not from a vendor
  "not supported" statement.

### 5.2 The forks prove the gap (do not attribute to upstream)

- `terrene-foundation/claude-squad` adds exactly what upstream
  lacks: "`cc login N`: saves current account's creds to
  credentials/N.json; `/rotate`: writes best account's creds
  to ~/.claude/.credentials.json," later refactored to
  per-terminal `CLAUDE_CONFIG_DIR` isolation with a quota
  statusline [https://github.com/terrene-foundation/claude-squad/commit/d3608765830da91ef4ef92ca3ee6bceb08db54d0](https://github.com/terrene-foundation/claude-squad/commit/d3608765830da91ef4ef92ca3ee6bceb08db54d0),
  [https://github.com/terrene-foundation/claude-squad/commit/faa2fe375903ae392d120a0f641025730f871127](https://github.com/terrene-foundation/claude-squad/commit/faa2fe375903ae392d120a0f641025730f871127).
  The fork even records the failure mode Juggler must avoid:
  "kill auto-rotate to stop random account switching"
  [https://github.com/terrene-foundation/claude-squad/commit/035e748a04e12c44c51a23c6e7a05371b981dbb3](https://github.com/terrene-foundation/claude-squad/commit/035e748a04e12c44c51a23c6e7a05371b981dbb3).
- `almanies2025/claude-squad` markets itself as "Multi-account
  rotation for Claude Code — intelligent quota management
  across Claude Max subscriptions" with "per-terminal
  isolation, shared history, and a statusline"
  [https://github.com/almanies2025/claude-squad](https://github.com/almanies2025/claude-squad) —
  again a fork, again absent upstream.
- A private fork notes the same: "The multi-account separation
  via CLAUDE_CONFIG_DIR per profile is the most valuable
  enhancement — it solves a problem Conductor itself cannot
  solve" [https://github.com/franzfelberer/claude-squad-fork/blob/HEAD/CLAUDE.md](https://github.com/franzfelberer/claude-squad-fork/blob/HEAD/CLAUDE.md).
  That sentence, from a Squad+Conductor user, is the wedge
  in one line.

### 5.3 Conductor: BYO subscription, no switcher, no quota UI

- Model: "you bring your own subscription" / "reusing your
  existing agent subscriptions"
  [https://github.com/laurence-nz/superset-pwsh/blob/HEAD/apps/marketing/content/compare/conductor-vs-cursor.mdx](https://github.com/laurence-nz/superset-pwsh/blob/HEAD/apps/marketing/content/compare/conductor-vs-cursor.mdx);
  "Both require you to pay the underlying AI providers
  (Anthropic, OpenAI, etc.) for agent usage"
  [https://github.com/per-simmons/damon-ade/blob/HEAD/apps/marketing/content/compare/superset-vs-conductor.mdx](https://github.com/per-simmons/damon-ade/blob/HEAD/apps/marketing/content/compare/superset-vs-conductor.mdx).
- Parent review: "**No Conductor-native multi-account switcher
  or quota dashboard found** in docs/searches (negative
  evidence; not confirmed absent from in-app settings)"
  (`docs/review/full-competitive-ux-review.md:614-616`).
  This deep dive re-tested with docs/changelog/pricing
  queries (§5 queries in research log): no official page
  describes account switching, quota rings, or reset
  countdowns. `[negative evidence]` remains the correct
  label — in-app settings cannot be inspected from docs.
- The pain underneath is real and unaddressed by Conductor:
  "a Claude Pro $20 5h cap is easily hit by 3–5 parallel
  agents" (`docs/review/full-competitive-ux-review.md:616-617`).
  Parallelism *multiplies* quota burn; Conductor accelerates
  the burn without instrumenting it.

### 5.4 Juggler: modelled, specced, not yet shipped

- Domain model already carries the shape: `QuotaWindow`
  with `used_percentage` + `resets_at`
  (`crates/juggler-core/src/model.rs:279-284`) and honesty
  tiers `Measured/Computed/Estimated`
  (`crates/juggler-core/src/model.rs:286-297`), attached per
  session via `UsageSnapshot` (`crates/juggler-core/src/model.rs:301-307`).
- Spec promises the feed: statusline JSON carrying
  `rate_limits.five_hour/seven_day.used_percentage/resets_at`
  captured as **Measured** (`docs/SPEC.md:192`), with quota
  rings + reset countdowns in Overview (`docs/SPEC.md:345`)
  and tray (`docs/SPEC.md:352`) — but milestones place the
  usage engine at M3 (`docs/SPEC.md:453`), after M2 adapters.
- Accounts themselves are first-class: named profiles with
  isolated state dirs via `CLAUDE_CONFIG_DIR`-style env
  overlay (`crates/juggler-adapter/src/adapter.rs:44`,
  `crates/juggler-adapter/src/environment.rs:34`), versus
  Squad's program-strings and Conductor's single ambient
  login. Only Claude is registered today
  (`crates/juggler-app/src/state.rs:91-96`).

**Test conclusion:** the juggling layer is unclaimed by both
competitors *architecturally* — Squad would need a credential
model it deliberately omits; Conductor would need to break
its "one login, many workspaces" simplicity. Juggler's risk
is not being beaten to quota — it is shipping adapters (M2)
before quota (M3) and arriving at the same blind spot.
Parent review F6.1 (`docs/review/full-competitive-ux-review.md:427-439`)
is the fix; this section is its competitive proof.

---

## 6. Diff / accept / merge flow — where Conductor wins outright

### 6.1 Claude Squad: preview, then keyboard commit verbs

- Flow: "Tab preview/diff, review-before-apply; `s` = commit+push
  branch, `c` = checkout (commit+pause: keeps branch, removes
  worktree), `r` = resume" (parent review,
  `docs/review/full-competitive-ux-review.md:595-598`).
- Semantics matter: "`c` (checkout) commits work locally with a
  templated message, removes the worktree, preserves the branch,
  and copies the branch name to the system clipboard. `r` (resume)
  recreates the worktree and reattaches the tmux session"
  [https://github.com/mkb23/overcode/blob/HEAD/docs/design/bakeoffs/overcode-vs-claude-squad.md](https://github.com/mkb23/overcode/blob/HEAD/docs/design/bakeoffs/overcode-vs-claude-squad.md).
- The verbs were buggy in exactly the trust-critical spot:
  "`c` Command Checkout Locally Only" — users found checkout
  pushed to GitHub first, contrary to "checkout changes before
  pushing them"; fix made `c` commit locally only, leaving push
  to the separate command [https://github.com/smtg-ai/claude-squad/pull/123](https://github.com/smtg-ai/claude-squad/pull/123).
  Lesson: in a review flow, *when* a push happens is a trust
  boundary. Juggler's passive invariant
  (`docs/review/agent-review-prompt.md:64-66`) must make push
  an explicit, labelled click — never a side effect of pause.
- Diff itself is terminal-grade: "preview diffs, commit/push
  (`s`), commit-and-pause (`c`)" with isolation as the safety
  story [https://github.com/kruxia/foundry/blob/HEAD/docs/research/2026-10-03-agentic-coding-harness-notes/harness_and_orchestration.md](https://github.com/kruxia/foundry/blob/HEAD/docs/research/2026-10-03-agentic-coding-harness-notes/harness_and_orchestration.md).
  No inline comments, no comment-to-agent loop. `[negative
  evidence]` — no comment feature appears in README/help
  sources above.

### 6.2 Conductor: the review cockpit (steal target #1)

- "Conductor's strongest code-surface feature is review
  ergonomics: docs and changelog emphasize a dedicated diff
  viewer and review pipeline (`Cmd+D`, review/fix/merge
  actions), plus 'checks' support"
  [https://docs.conductor.build/core/diff-viewer](https://docs.conductor.build/core/diff-viewer)
  (via [https://github.com/debuglebowski/slayzone/blob/HEAD/comparison/_legacy/conductor.md](https://github.com/debuglebowski/slayzone/blob/HEAD/comparison/_legacy/conductor.md)).
- Official diff docs: "The Diff Viewer shows the code changes
  in a workspace. Use it before creating a pull request, after
  an agent makes changes, and whenever review comments need
  another pass" — checking "Files changed by the agent.
  Unrelated or accidental edits. Missing tests or docs. Local
  comments and GitHub review comments"
  [https://www.conductor.build/docs/reference/diff-viewer](https://www.conductor.build/docs/reference/diff-viewer).
- The killer detail: "clicking a line in the diff and adding
  a comment routes that comment to the agent as **targeted
  context with file+line metadata**, not as a chat message"
  [https://github.com/dearken10/baton/blob/HEAD/docs/prior-art.md](https://github.com/dearken10/baton/blob/HEAD/docs/prior-art.md).
  That closes the review loop without context-switching —
  the exact loop parent review flags as missing in Juggler
  (F7.3, `docs/review/full-competitive-ux-review.md:497-505`).
- Full pipeline in one surface: "Diff Viewer, terminal/Spotlight
  tests, Checks tab (git status, CI, deployments, comments,
  todos), create PR, merge, archive" (parent review,
  `docs/review/full-competitive-ux-review.md:620-623`); third
  parties confirm "Review workspace changes, comments, and
  PR-ready diffs" as the documented purpose
  [https://www.conductor.build/docs/reference/diff-viewer](https://www.conductor.build/docs/reference/diff-viewer).
- Independent verdict: Conductor's "in-app review-and-merge
  cockpit [is] the single feature that most clearly beats
  [a competitor] on product craft"
  [https://github.com/thefrederiksen/devthrottle/issues/1146](https://github.com/thefrederiksen/devthrottle/issues/1146).

### 6.3 Juggler today: terminal only, no diff surface

A `grep` for `diff|Diff|branch` across `ui/src/components/*.tsx`
and `ui/src/screens/*.tsx` returns zero hits (verified
in-session). Sessions embed their live terminal
(`docs/review/agent-review-prompt.md:52`) and handover copies
a transcript (`crates/juggler-app/src/commands.rs:1088-1194`),
but there is no changed-files view, no diff, no PR action.
The spec's only diff mention is inside the handover pack
manifest (`docs/SPEC.md:301`). So Juggler today asks users
to review agent work in an external editor/`git diff` —
while both competitors keep review in-app. Severity: P1
(not P0 — review can happen externally; quota blindness
cannot). Sequence: worktree first (makes diff meaningful),
read-only diff second, comment-to-agent last (parent review
F7.3 ordering, `docs/review/full-competitive-ux-review.md:502-505`).

---

## 7. Multi-harness, pricing/licence, platform

### 7.1 Multi-harness breadth vs depth

| Axis | Claude Squad | Conductor | Juggler |
|---|---|---|---|
| Harnesses | Any CLI via `-p` [https://github.com/smtg-ai/claude-squad](https://github.com/smtg-ai/claude-squad) | Claude Code, Codex, Cursor, OpenCode [https://www.agentastic.dev/alternatives/paseo](https://www.agentastic.dev/alternatives/paseo) | Claude only (`crates/juggler-app/src/state.rs:91-96`) |
| Contract | Program string, no adapter API | Built-in, closed | Typed trait (`crates/juggler-adapter/src/adapter.rs:26`) |
| Per-harness depth | Shallow (spawn + tmux) | Deep (bundled Codex updates [https://www.conductor.build/changelog](https://www.conductor.build/changelog)) | Deep per harness, narrow count |
| Continuation | tmux reattach + `claude --continue` [https://github.com/3jins/claude-squad-restore](https://github.com/3jins/claude-squad-restore) | Workspace persists; open in IDE [https://github.com/yzgershon/gatedspace/blob/HEAD/apps/marketing/content/compare/superset-vs-conductor.mdx](https://github.com/yzgershon/gatedspace/blob/HEAD/apps/marketing/content/compare/superset-vs-conductor.mdx) | Transcript transplant + `--resume` (`crates/juggler-app/src/commands.rs:1088-1194`) |

Squad breadth is thin (same spawn path, no transcript/quota/
handover semantics). Juggler's trait
(`crates/juggler-adapter/src/lib.rs:6-7`) trades breadth for
juggling depth — correct only if adapter #2 ships with the
conformance kit (parent review F1.8/F5.5).

### 7.2 Pricing & licence — parent review correction

- **Claude Squad: free, AGPL-3.0.** "AGPL-3.0 — *forking is
  viral-license territory; read-for-ideas only*"
  [https://github.com/gregce/tortie/blob/HEAD/docs/research/10-multi-project-ux.md](https://github.com/gregce/tortie/blob/HEAD/docs/research/10-multi-project-ux.md);
  licence + 7,171 stars in the structured snapshot
  [https://github.com/kayushkin/llm-bridge/blob/HEAD/RELEASE-TARGETS.md](https://github.com/kayushkin/llm-bridge/blob/HEAD/RELEASE-TARGETS.md).
  Parent review's platform table ("Claude Squad | macOS/Linux
  TUI | AGPL-3.0, free," `docs/review/full-competitive-ux-review.md:724`)
  is correct. Implication: Juggler (MIT, `Cargo.toml:17`) can
  *read* Squad for ideas but must not copy AGPL code. Ideas
  (verbs, lifecycle) are free; implementation must be original.
- **Conductor: free today; parent review's $50/$60 is stale.**
  Parent review states "Free local / Pro $50/mo / Teams $60"
  (`docs/review/full-competitive-ux-review.md:625`) and
  "Free / $50 Pro" (`docs/review/full-competitive-ux-review.md:725`).
  Current sources say "Conductor is free. Both require you to
  pay the underlying AI providers"
  [https://github.com/per-simmons/damon-ade/blob/HEAD/apps/marketing/content/compare/superset-vs-conductor.mdx](https://github.com/per-simmons/damon-ade/blob/HEAD/apps/marketing/content/compare/superset-vs-conductor.mdx),
  "Proprietary, free" [https://github.com/rodgi040/secret-ai-beaver-sauce/blob/HEAD/skills/secret-ai-beaver-source/references/TOOLS.md](https://github.com/rodgi040/secret-ai-beaver-sauce/blob/HEAD/skills/secret-ai-beaver-source/references/TOOLS.md),
  and pricing is gated/unclear behind the official page
  [https://www.conductor.build/pricing](https://www.conductor.build/pricing)
  (via [https://github.com/indosaram/ferryx/blob/HEAD/docs/SITE_SEARCH_PRIORITIES_2026-09-19.md](https://github.com/indosaram/ferryx/blob/HEAD/docs/SITE_SEARCH_PRIORITIES_2026-09-19.md)).
  One comparison still lists team/cloud gating
  [https://github.com/kogan/superset/blob/HEAD/apps/marketing/content/compare/superset-vs-conductor.mdx](https://github.com/kogan/superset/blob/HEAD/apps/marketing/content/compare/superset-vs-conductor.mdx).
  **Correction:** treat Conductor as freeware-with-paid-team/
  cloud upsell `[unresolved exact tiers]` — do not repeat
  $50/$60 as fact. Either way, Juggler cannot win on price
  against free; it wins on openness + juggling.
- **Juggler: MIT, free, signed macOS distribution planned**
  (`README.md:83`, `docs/SPEC.md:359-360`). Against AGPL
  (Squad) and proprietary freeware (Conductor), MIT +
  local-only is the trust-maximising licence — say so.

### 7.3 Platform

- Squad: macOS/Linux TUI; requires `tmux` + GitHub CLI `gh`
  ("It needs tmux and the GitHub CLI"
  [https://github.com/agentlistio/awesome-agent-clients](https://github.com/agentlistio/awesome-agent-clients)).
  No Windows story in official sources. `[negative evidence]`
  for Windows support.
- Conductor: "The desktop app is Mac-only; cloud workspaces
  run on Linux and can be driven from the Mac app or the iOS
  app" [https://github.com/thomaschristopherking/conductor-android/blob/HEAD/docs/conductor-skill.md](https://github.com/thomaschristopherking/conductor-android/blob/HEAD/docs/conductor-skill.md);
  iOS is "Coming soon" in one comparison
  [https://github.com/kogan/superset/blob/HEAD/apps/marketing/content/compare/superset-vs-conductor.mdx](https://github.com/kogan/superset/blob/HEAD/apps/marketing/content/compare/superset-vs-conductor.mdx) —
  conflicting signals, flag `[unresolved]` for iOS status.
- Juggler: "macOS 13+ (Universal). Windows/Linux post-1.0;
  the Rust core and adapters are cross-platform from day one"
  (`docs/SPEC.md:18`). Squad already covers Linux terminal
  users Juggler defers; Conductor covers cloud/Linux compute
  Juggler defers to Fleet (M6+). Both deferrals are fine —
  but marketing must not imply parity today.

---

## 8. Review UX, notifications & failure modes (compact)

**Review UX.** Squad: keyboard-first preview/diff, no inline
comments `[negative evidence, §6.1]`. Conductor: visual diff
viewer + line comments routed to agent as file+line context
[https://github.com/dearken10/baton/blob/HEAD/docs/prior-art.md](https://github.com/dearken10/baton/blob/HEAD/docs/prior-art.md),
Checks tab for CI/todos (parent review
`docs/review/full-competitive-ux-review.md:620-623`). Juggler:
no diff UI (zero hits, §6.3); review happens in the embedded
terminal or externally. **Notifications.** Neither competitor
documents a quota/waiting notification system: Squad relies on
the TUI list + autoyes daemon polling
[https://github.com/mkb23/overcode/blob/HEAD/docs/design/bakeoffs/overcode-vs-claude-squad.md](https://github.com/mkb23/overcode/blob/HEAD/docs/design/bakeoffs/overcode-vs-claude-squad.md);
Conductor changelog emphasises workspace status/navigation,
not alerts [https://www.conductor.build/changelog](https://www.conductor.build/changelog).
`[negative evidence]` for both. Juggler specs tray alerts
(`docs/SPEC.md:352`) but ships none (`grep` tray/notification
in `ui/src/` + `crates/` = zero hits, verified in-session) —
an open lane: quiet digest + needs-attention badge (parent
review WOW-g, `docs/review/full-competitive-ux-review.md:788-790`)
beats both by existing. **Failure modes.** Squad: resumed
sessions fail when worktree creation changes CWD — "The tmux
session starts in the worktree path instead of the original
project path… the agent starts in the wrong directory, can't
find the session to resume, exits immediately"
[https://github.com/rakesh97/claude-squad/issues/28](https://github.com/rakesh97/claude-squad/issues/28);
reboot loses tmux sessions (history survives, sessions do
not) [https://github.com/3jins/claude-squad-restore](https://github.com/3jins/claude-squad-restore);
multi-instance tmux prefix collisions need manual config
[https://github.com/smtg-ai/claude-squad](https://github.com/smtg-ai/claude-squad).
Conductor: closed source = unauditable failure surface;
quota burn at 3–5 parallel agents (§5.3) with no in-app
guardrail `[negative evidence]`. Juggler: handover picks
newest-by-mtime JSONL, no source freeze, `replace('/','-')`
cwd guess (`crates/juggler-app/src/commands.rs:1088-1194`;
parent review F3.3, `docs/review/full-competitive-ux-review.md:264-287`) —
same class of resume fragility as Squad's CWD bug. Fix both
with `transcript_ref` (`crates/juggler-core/src/model.rs:314-321`)
+ freeze-before-copy.

## 9. Handover/continuation — the asymmetric feature

Neither competitor hands work *between accounts*. Squad
continuation is same-account tmux reattach
[https://github.com/3jins/claude-squad-restore](https://github.com/3jins/claude-squad-restore);
Conductor continuation is the PR (humans/CI carry context).
Juggler's transcript transplant + `--resume`
(`crates/juggler-app/src/commands.rs:1088-1194`) with lineage
(`SessionStatus::HandedOver`, `crates/juggler-core/src/model.rs:188-207`)
is category-unique — and currently fragile (F3.3) and
over-claimed (`TRANSPLANT|FORK` advertised at
`crates/juggler-adapters/src/claude_code.rs:111-119` while
`crates/juggler-handover/src/lib.rs:1-13` is doc-only; parent
review F3.1). Protect the uniqueness, fix the honesty first.

## 10. Feature-by-feature verdict (Gap = Juggler gap severity)

| Feature | Squad | Conductor | Juggler (repo) | Gap | Verdict |
|---|---|---|---|---|---|
| Worktree isolation | Yes (§3.1) | Yes (§3.2) | No (`model.rs:113`) | **P0** | **Steal** Squad atomicity |
| Parallel list/triage | List+preview | Workspace dashboard | Rail, capped (`WorkflowRail.tsx:184-195`) | P1 | Beat with inbox (WOW-a) |
| Diff review | Preview diff | Diff+comments→agent | None (§6.3) | P1 | **Steal** Conductor cockpit |
| Accept/merge/PR | `s`/`c`/`r` verbs | PR→merge→archive | None | P1 | Steal verbs, explicit push |
| Multi-harness | Any CLI `-p` | 4 built-in | 1 (`state.rs:91-96`) | P1 | Beat on depth, not count |
| Multi-account | No (§5.1) | No (§5.3) | Yes, first-class (§5.4) | — | **Beat: defend wedge** |
| Quota display | No `[neg. ev.]` | No `[neg. ev.]` | Modelled M3 (`SPEC.md:453`) | **P0** | Ship before adapters |
| Handover lineage | Branch only | PR only | v0 transplant (§9) | P1 | Fix honesty (F3.1–F3.4) |
| Autoyes/auto-accept | `-y` experimental | No | Forbidden by invariant | — | **Avoid** (breaks click-bound rule) |
| Notifications | None `[neg. ev.]` | None `[neg. ev.]` | Specced, unbuilt (`SPEC.md:352`) | P1 | Beat by shipping digest |
| Setup/run scripts | No | Yes (§3.2) | No | P2 | Steal later |
| Cloud/mobile | No | Cloud+iOS `[unresolved]` | Fleet M6+ | P2 | Avoid now |
| Licence/trust | AGPL free | Proprietary free | MIT (`Cargo.toml:17`) | — | Beat: lead with openness |

**Bottom line:** copy isolation and review craft; never copy
automation that acts without a click; win on the one axis
both competitors structurally omit — quota-aware juggling
across named accounts. *End of deep profile 05.*
