# AI Juggler — Durable Task System

> Purpose: every settled product decision is broken into detailed,
> researched, evidence-gated tasks that survive context compaction,
> context loss and agent handover. A new agent with zero chat history
> must be able to start from this directory alone.
> Status of this system: all tasks are `TODO`. Nothing here is done
> until its evidence exists (see "Definition of done" below).

## Start here (cold-agent reading order)

1. [AGENT-HANDOVER.md](AGENT-HANDOVER.md) — state snapshot, invariants,
   claiming protocol, compaction-recovery steps.
2. [../review/decision-log.md](../review/decision-log.md) — settled
   decisions D-01..D-15. Do not re-litigate them; supersede only via a
   new recorded decision.
3. [TASK-CATALOG.md](TASK-CATALOG.md) — every task ID, epic, priority.
4. The epic file for the task you claim (E1-E10 below).
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
| [E8-validation-visual-performance-quality.md](E8-validation-visual-performance-quality.md) | Evidence, visual, a11y, perf, gates | AJ-E801-E810 |
| [E9-docs-adrs-and-plan-rewrite.md](E9-docs-adrs-and-plan-rewrite.md) | Stage-1 docs, ADRs, plan rewrite | AJ-E901-E910 |
| [E10-derived-workflows-and-beta.md](E10-derived-workflows-and-beta.md) | Derived workflows, safety nets, beta ops | AJ-E1001-E1011 |
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

- `TODO` — not started (initial state of all 107 tasks).
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

## Stage boundary (read before starting anything)

D-15 limits the current approved stage to documents (E9) and research
records (E6/E909 logs). Implementation epics (E1-E8, E10) are fully
specified now so no reasoning is lost, but executing them requires a
separate maintainer approval per stage. Specifying a task here is not
authorisation to build it.

## Evidence root

All proof lives under `docs/qa/evidence/<task-id>/` (contract in
AJ-E801): manifest with requirement, command, threshold, observed
result, PASS/FAIL, commit SHA, date, reviewer. No evidence may live
only in chat, PR descriptions or an agent's memory.
