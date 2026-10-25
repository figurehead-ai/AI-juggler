# AI Juggler — Durable Task System

> Purpose: every settled product decision is broken into detailed,
> researched, evidence-gated tasks that survive context compaction,
> context loss and agent handover. A new agent with zero chat history
> must be able to start from this directory alone.
> Each task carries its own status in its epic file. Nothing here is done
> until its evidence exists (see "Definition of done" below).

## Start here (cold-agent reading order)

1. [AGENT-HANDOVER.md](../internal/AGENT-HANDOVER.md) — state snapshot, invariants,
   claiming protocol, compaction-recovery steps.
2. [../review/decision-log.md](../review/decision-log.md) — settled
   decisions D-01..D-40. Do not re-litigate them; supersede only via a
   new recorded decision.
3. [TASK-CATALOG.md](TASK-CATALOG.md) — every task ID, epic, priority.
4. The epic file for the task you claim (E1-E23 below).
5. Source research cited inside that task (pass2/deep reports, SPEC,
   code at the cited lines) — read before editing anything.

## File map

| File | Epic | Tasks |
|---|---|---|
| [E1-terminology-and-naming.md](E1-terminology-and-naming.md) | Providers/Accounts/Instances rename | AJ-E101-E108 |
| [E2-switching-engine.md](E2-switching-engine.md) | Switch Graph, pack, fidelity, switch-back | AJ-E201-E210 |
| [E3-smart-onboarding.md](E3-smart-onboarding.md) | Smart scan, inference, six flows | AJ-E301-E309 |
| [E4-indicators-and-inspector.md](E4-indicators-and-inspector.md) | Indicator row, Right Inspector, signals | AJ-E401-E415 |
| [E5-process-ownership-and-cleanup.md](E5-process-ownership-and-cleanup.md) | Spawn ledger, zombies, Cleanup Centre | AJ-E501-E512 |
| [E6-quota-cost-and-billing-research.md](E6-quota-cost-and-billing-research.md) | Quota/cost engines + billing API research | AJ-E601-E611 |
| [E7-fleet-at-launch.md](E7-fleet-at-launch.md) | Fleet at launch (beta) | AJ-E701-E711 |
| [E8-validation-visual-performance-quality.md](E8-validation-visual-performance-quality.md) | Evidence, visual, a11y, perf, gates | AJ-E801-E811 |
| [E9-docs-adrs-and-plan-rewrite.md](E9-docs-adrs-and-plan-rewrite.md) | Stage-1 docs, ADRs, plan rewrite | AJ-E901-E910 |
| [E10-derived-workflows-and-beta.md](E10-derived-workflows-and-beta.md) | Derived workflows, safety nets, beta ops | AJ-E1001-E1025 |
| [E11-usage-statistics.md](E11-usage-statistics.md) | Opt-in usage statistics | AJ-E1101-E1109 |
| [E12-optional-account.md](E12-optional-account.md) | Optional Figurehead account (two paths) | AJ-E1201-E1207 |
| [E13-codex-adapter.md](E13-codex-adapter.md) | Codex, the second Provider | AJ-E1301-E1306 |
| [E14-machine-health.md](E14-machine-health.md) | Machine health and resource monitoring | AJ-E1401-E1410 |
| [E15-spec-gaps.md](E15-spec-gaps.md) | Spec gaps: what the specs promised and no task covered | AJ-E1501-E1544 |
| [E16-more-tools.md](E16-more-tools.md) | More tools: OpenCode, Kimi, Grok, ZCode, Kiro, Hermes, Muse, Cursor, desktop apps | AJ-E1601-E1613 |
| [E17-agent-pools.md](E17-agent-pools.md) | Agent pools and the agent bridge | AJ-E1701-E1705 |
| [E18-agent-messaging.md](E18-agent-messaging.md) | Messaging between agents | AJ-E1801-E1805 |
| [E19-agent-tasks.md](E19-agent-tasks.md) | Task management shared by pools | AJ-E1901-E1906 |
| [E20-engineering-and-security.md](E20-engineering-and-security.md) | Engineering and security foundations | AJ-E2001-E2019 |
| [E21-desktop-first.md](E21-desktop-first.md) | Desktop first: what's open, where, and one click to it | AJ-E2101-E2110 |
| [E23-session-runtime.md](E23-session-runtime.md) | A runtime per tool: driven sessions, exact state, answers from AI Juggler | AJ-E2301-E2310 |
| [CRITIC-CHECKS.md](CRITIC-CHECKS.md) | Cross-epic audit of this system | — |

## Task format (every task, no exceptions)

Each task carries: Goal; User story; numbered testable SHALL
Requirements; UI/UX reasoning tied to a named guideline (Nielsen/NN-g,
Apple HIG, WCAG) plus exact states/copy/shortcuts; a numeric
Performance budget; Quality gates; an Evidence/validation proof with
exact command/fixture/live step, pass threshold and storage path;
Visual testing requirements (viewports, states, comparison, a11y);
real Files to touch; Research basis with file:line or URL; and a
Handover note naming traps. If any field is missing, the task is
defective — fix the task before starting work (see CRITIC-CHECKS.md).

## Status workflow

- `TODO` — not started (every task's initial state).
- `IN-PROGRESS (agent, date)` — claimed; record who/when in the status.
- `BLOCKED (reason, task-ID)` — only with the blocking task or decision
  named.
- `DONE-EVIDENCED` — only when the task's Evidence/validation proof
  exists at its stated path and passes at its stated threshold, and
  its Visual testing artifacts are stored. Code merged without
  evidence is not done.

Flip status only in the same PR that adds the evidence link. Never
delete a task; supersede it with a new ID and a forward link.

## Settled decisions (one line each; full text in the decision log)

- D-01 Thesis: Switch → See → Clean; quota is a trigger, not headline.
- D-02 Full phased rename to Providers / Accounts / Instances.
- D-03 Hybrid switching: 1-click when proven, wizard when lossy.
- D-04 Ship all switching in beta now; validation runs in parallel.
- D-05 Switch-back: resume original + one delta; never copy-back.
- D-06 Full six-tab Right Inspector, honest fields only.
- D-07 Quality = measurable signals; no composite score.
- D-08 Cleanup: notify-only default; Clean-safe opt-in per Account.
- D-09 Tagline A + subtitle B.
- D-10 "Profile" demoted to Launch settings.
- D-11 Cross-Provider switching claimed only as labelled beta.
- D-12 Subscription expiry user-entered/optional; billing-API
  research (E6/E909) mandatory before assuming no source exists.
- D-13 Fleet ships at launch; architect for it from day one.
- D-14 Pack secret handling: user's remembered choice, global with
  per-Provider override; never a hidden product default.
- D-15 Stage 1 is documents only; code needs separate approval.
- D-16 Two themes, Light and Dark (System offered); toggle in Settings.
- D-17 Screen capture permitted through one vendored, audited init shim.
- D-18 Optional Figurehead account: two paths (desktop / cloud); with no
  account, zero calls to figurehead.ai; every cloud feature free at launch.

## Stage boundary (read before starting anything)

Stage 1 (documents, D-15) is complete. Stage 2 (implementation of every
epic) was approved by the maintainer on 2026-10-13 and is running; see
AGENT-HANDOVER.md for the current snapshot.

## Evidence root

All proof lives under `docs/qa/evidence/<task-id>/` (contract in
AJ-E801): manifest with requirement, command, threshold, observed
result, PASS/FAIL, commit SHA, date, reviewer. No evidence may live
only in chat, PR descriptions or an agent's memory.
