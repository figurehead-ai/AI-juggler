# AI Juggler — Autonomous build runbook

This is the operating contract for unattended build runs (scheduled agent sessions). It exists so that autonomous work is disciplined, reviewable, and indistinguishable in quality from attended work. **All persistent state lives in this repo (issues, docs, code) — never in chat history.**

## Context (read first, every run)

- Repo: GitHub `figurehead-ai/AI-juggler` (public). Work from a local clone with `gh` authenticated as a maintainer.
- Plan: [EPICS.md](./EPICS.md) (tasks & gates) · Standards: [QUALITY.md](./QUALITY.md) (DoD — every task) · Requirements: [SPEC.md](../SPEC.md).
- Branch protection is ON for `main`: required CI checks, linear history, no force-push. All work goes through PRs.

## Operating loop (each run)

1. `git fetch && git checkout main && git pull` (if `main` is broken, fixing it becomes the run's top priority).
2. Find the current milestone: the lowest `m<N>` label that still has open issues.
3. Pick the next task: the lowest-numbered open issue labeled `type:task` in that milestone.
4. Branch `feat/m<N>-<id>-<slug>` from `main`; implement against SPEC §x + issue acceptance criteria; satisfy every DoD item in QUALITY.md — including tests, fixtures, docs, and CHANGELOG.
5. Self-review the full diff against QUALITY.md §9 before opening a PR.
6. Open a PR (template checklist completed, "Closes #N"), wait for CI green, then merge **squash** + delete branch.
7. Close-out: comment evidence on the task issue (what was validated, links), check its boxes, let the close happen via the PR.
8. Repeat until the run's time is used, a stop condition hits, or the milestone's tasks are done.
9. End of run: leave `main` green. Post a run summary **only** as a reply on the milestone tracking issue if the milestone completed or a stop condition fired; otherwise keep noise to zero (task issues already carry evidence).

## Gates — the mandatory human checkpoints

When every `type:task` issue of the current milestone is closed:

1. Execute the gate: full validation per EPICS.md (automated + functional, every acceptance criterion evidenced).
2. Perform the quality audit for the milestone's diff vs QUALITY.md; fix findings first.
3. Write `docs/qa/gate-MX.md` from the QUALITY.md §8 template; commit; link it in a comment on the milestone tracking issue (`type:gate`).
4. **STOP.** The user's confirmation on that issue is required before the next milestone's task issues are created or any of its work starts. Never self-confirm a gate, never close a `type:gate` issue.

## Stop conditions (report, don't stall silently)

A run stops immediately and reports to the user when:

- a milestone gate is reached (see above);
- a **UI/UX design checkpoint** is due (from M1): the design direction at M1-1 and each major screen as it lands are presented with screenshots and paused for maintainer review before broad implementation continues — the bar is a super-modern, highly usable UI aligned with the app's workflow;
- credentials/secrets only the user can provide are needed (e.g., Apple Developer / signing keys / GitHub Environments — M5+);
- a genuine product decision only the user can make arises (open a `question` issue with options + a recommendation, then stop);
- CI or repo infrastructure is broken beyond fix-forward in one run;
- a vendor-format change invalidates spec assumptions (record findings in the relevant research doc, open an issue, stop if it blocks the current task).

A stop report states: what was completed (issues closed), what's next, and exactly what is needed from the user.

## Hard rails (always)

- No force-push, no direct commits to `main`, no bypassing branch protection.
- No secrets in code, logs, issues, or PRs — ever.
- No DoD exceptions; if scope pressure appears, split the task or stop — never lower the bar.
- No dependency upgrades outside a task's scope; no touching lockfiles casually.
- Compliance guardrails (ADR-0003) are non-negotiable in code and UX copy.
- PRs stay small (≤ ~400-line guideline); large work lands as a stack of small PRs.
- If two runs ever overlap, issues are the lock: before starting, re-check the issue is still open and unassigned-in-progress.

## Cadence & control

- The loop is driven by a scheduled automation (every 30 minutes) plus an immediate kickoff run; each run re-reads this file, so behavior changes only by changing this file.
- The user can pause/resume/stop the automation at any time (Automations page, or just say so) — runs stop with the schedule; in-flight work finishes or is abandoned on a branch (never half-merged).
