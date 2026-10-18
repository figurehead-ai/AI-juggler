# Second-opinion review prompt (for an independent AI coding agent)

Copy everything below the line into the reviewing agent. It assumes the
agent is running in this repository, on `main`, with read access to the
same machine (it must NOT modify anything — see ground rules).

---

You are giving an independent second-opinion review of **AI Juggler**, an
open-source product being built in this repository (`~/Dev/AIJuggler`,
branch `main`). Your job: find what we missed, challenge our priorities,
and stress-test the plan — as a skeptical senior product engineer + staff
architect would. You are the reviewer; the team (another AI agent, plus
the human maintainer) will read and act on your findings.

## Ground rules (hard)

- **Read-only.** Do not modify files, do not commit, do not create branches.
- **Never touch live user data**: `~/.claude`, `~/Library/Application
  Support/ClaudeCodeProfiles/` (managed by another running tool, t3.codes),
  or `~/Library/Application Support/AI Juggler/` (live app state). Reading
  file *metadata* (names/sizes) inside this repo is fine; do not read
  credential files anywhere, ever.
- Do not run the desktop app interactively, do not start logins, do not
  kill processes. You MAY run the offline gates: `just verify`
  (fmt/clippy/tests), `cargo test --workspace`, and `pnpm --dir ui build`.
- Use `gh issue list --state all` and `gh pr list --state merged` for the
  work history if the `gh` CLI is authenticated.

## What we are building, and why

**The problem (verified by research in `docs/research/`):** Developers who
pay for multiple AI coding subscriptions (Claude Code, Codex, ZCode…) have
no sanctioned way to run several accounts in parallel on one machine. The
community workaround is shell aliases + `CLAUDE_CONFIG_DIR` + terminal
sprawl. Quota is opaque (Claude's 5h/weekly windows etc.) and runs out
mid-task, killing momentum. Sessions are silos: no unified view of what's
running, no stuck detection, no sanctioned way to continue a conversation
on another account.

**The product:** a free, open-source, native desktop app (Rust + Tauri 2,
macOS-first) that *supervises* the harnesses you already have installed —
never a middleman. Its name is its core promise: **juggling** — run
multiple isolated accounts in parallel per project, and when one account
runs dry, continue the same work on another with one click (handover).
Core workflows it must nail, in order:

1. **Onboarding with zero knowledge** — the app scans the machine
   (including other manager tools' profile roots), imports existing setups
   read-only, guides the vendor's own login in an embedded terminal.
2. **The daily loop** — left rail: Projects / Instances / Sessions; select
   anything → its panel loads, session panels embed their live terminal.
   New session = pick a recent project / native folder picker / drag-drop,
   never typed paths.
3. **The juggle** — per project: see which account is on it, its history
   across accounts, and "Continue on another account" (transcript
   transplant + `claude --resume`), with lineage recorded.
4. Later milestones: quota/usage visibility, machine resource awareness,
   multi-machine fleet, Codex/ZCode adapters, desktop-class harnesses.

**Non-negotiable invariants (product-level, check we actually hold them):**
- Never automate or touch vendor logins/credentials (metadata-only
  detection; the vendor's own flow in an embedded terminal).
- All automation is passive (monitor/alert/suggest); every state-changing
  action is an explicit user click. Single exception: opt-in per-profile
  crash auto-restart (same profile only), off by default.
- Zero writes outside managed dirs; user's pre-existing installs
  (`~/.claude`, t3 profiles) are never modified (imports are byte-copies,
  manifest-verified).
- No traffic proxying/MITM; spawn unmodified real CLIs; everything local.

## Where everything is

Read in this order (all paths from repo root):

1. `README.md`, `docs/PROPOSAL.md` — the why (goals G1–G8, non-goals)
2. `docs/SPEC.md` — the contract (§3 domain, §4 adapters, §5 supervisor,
   §7 handover tiers T1–T4, §9 UX/IA, §12 storage + onboarding, §11 fleet)
3. `docs/plan/EPICS.md` — milestones M0–M8 and acceptance criteria
4. `docs/plan/QUALITY.md`, `docs/plan/RUNBOOK.md` — standards + process
5. `docs/adr/0001`–`0007 — every significant decision, with alternatives
6. `docs/research/*.md` — original evidence base
7. `docs/qa/gate-M1.md` — current milestone gate status
8. `CHANGELOG.md` + merged PRs — what actually landed, in order

Code map: `crates/juggler-core` (domain+SQLite), `crates/juggler-adapter`
(the `HarnessAdapter` trait contract), `crates/juggler-adapters` (Claude
Code adapter + scriptable fake harness), `crates/juggler-supervisor`
(PTY/piped processes, stop ladder, journal, crash adoption),
`crates/juggler-app` (Tauri commands/events), `ui/` (React 18 + TS +
Tailwind v4; screens in `ui/src/screens`, rail in
`ui/src/components/WorkflowRail.tsx`). Tests live beside each crate,
including operational runners marked `#[ignore]` (real-machine dogfood).

## Honest current state (as of this prompt)

Built and merged (CI-gated, 82+ tests): full Rust engine; Claude Code
adapter (isolation, login detection, transcript parsing); supervisor with
embedded xterm terminals; instance-centric UI (Projects/Instances/Sessions
rail + panels); smart onboarding (scan incl. t3.codes roots, read-only
import, guided login); real project history backfilled from imported
transcripts; handover v0 (transcript transplant + `--resume`, lineage);
single-instance + crash reconciliation; get-ready home. The maintainer's
five real t3.codes accounts are imported (copies) and their real projects
surface in the app.

Deliberately not built yet (placeholders are labeled in-UI): Usage/quota
dashboards (M2–M4), machine health gauges (M3), Fleet (M6+), Codex/ZCode
adapters (M2), desktop-class harness adapters, signed releases (M5+).
Handover v0 carries the transcript; fidelity beyond the JSONL (if the
vendor keeps state elsewhere) is unverified until first real login.

## What we want from you (the review)

Answer with **evidence** (file paths / issue numbers) and **priority**
(P0 blocks the core promise, P1 should land before v0.1-beta, P2 later).
Be specific and ruthless; "looks good" is useless to us.

1. **Missing features vs. the vision.** Read PROPOSAL goals G1–G8 and
   SPEC §1–§12 against what exists. What did the *plans themselves* fail
   to include that a multi-subscription developer will obviously need?
   (Think: quota alerts, account health, session cleanup, updates,
   first-run edge cases, multi-repo workflows, costs, safety rails…)
2. **Workflow-level critique.** Walk the intended daily loop mentally
   (onboard → project → parallel accounts → quota wall → continue on
   another account). Where does it friction, confuse, or silently fail?
   Where does our current UI promise something the engine doesn't do?
3. **Handover realism.** SPEC §7 defines tiers T1–T4. Is transplanting
   the JSONL + `--resume` (current v0) sound for Claude Code? What breaks
   (context window, todo/state files, sidechains/subagents, worktrees)?
   What's the minimum honest version we should ship?
4. **Compliance audit.** Check the invariants above against the actual
   code (adapters, supervisor, import paths). Any place we read too much,
   write too far, or automate what should be a click?
5. **Architecture/quality risks.** Anything in the crate layout, data
   model (projects/instances/sessions), storage migrations, or the
   adapter contract that will bite at 10+ accounts / 20+ sessions /
   3 harnesses? Any single points of failure in the supervisor design?
6. **Roadmap sanity.** Given limited build capacity, is M2(Codex/ZCode)
   before quota-visibility (M4) the right order for the core promise?
   What would you reorder and why?
7. **Competitive angle.** We know of t3.codes (unified agent GUI),
   aisw/claude-swap (CLI switchers). Based on our docs, what table-stakes
   capability are we missing that users of those tools will expect?

Output format: a markdown report with (a) an executive summary (≤10
bullets), (b) findings grouped by the seven questions, each with priority
+ evidence + a concrete recommendation, (c) a final "features we missed"
checklist sorted by priority. Do not restate our docs back to us — only
add judgment.
