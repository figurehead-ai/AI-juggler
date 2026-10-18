# Agent Handover — Read This First, Trust Nothing Else From Memory

> Written for an agent (or human) arriving with zero context: after a
> compaction, a context loss, or a team change. If chat history and
> this file disagree, this file and the decision log win.

## 30-minute cold start

1. Read [README.md](README.md) in this directory (5 min).
2. Read [../review/decision-log.md](../review/decision-log.md)
   D-01..D-18 (10 min). These are settled with the maintainer. Do not
   re-ask, re-litigate or "improve" them; a change requires a new
   recorded decision with maintainer approval.
3. Skim [TASK-CATALOG.md](TASK-CATALOG.md); note the epic of your task.
4. Read your task in full, then read every source it cites under
   **Research basis** (repo files at cited lines, pass2/deep sections).
5. Check current repo state yourself (`git status`, the cited code)
   before believing any "already done" claim — including claims in the
   research reports, which describe the repo as of 2026-10-12/13.

## Current state snapshot (verify, do not trust blindly)

Updated 2026-10-18. Re-verify with `git log --oneline -20` and
`grep -h "^### AJ-" docs/tasks/E*.md | grep -c "DONE-EVIDENCED"`.

- **Repository:** public, `figurehead-ai/AI-juggler` (MIT). It started
  from a clean snapshot on 2026-10-18; issue/PR numbers in older docs
  refer to the pre-public development repository. **Everything here is
  public** — never commit personal data (real account names, e-mails,
  home paths, hostnames, real-desktop screenshots, tokens) or server
  internals of figurehead.ai (see CONTRIBUTING.md).
- Decision log: D-01..D-18. D-16 dual themes, D-17 screen-capture
  shim, D-18 optional Figurehead account (two paths; ADR-0014).
- Stage 2 (implementation) is approved and running.
- Progress: 89/114 DONE-EVIDENCED — E1, E2, E3, E4, E5, E6, E9
  complete; E7 8/11 (AJ-E701..E705, AJ-E707, AJ-E709, AJ-E711); E8 3/10; E10 not started; E12 3/7
  (AJ-E1201..E1203: decision, account client, Settings). AJ-E708 is
  IN-PROGRESS: its sync substrate is evidenced; the Diverged state and
  the conflict sheet land with AJ-E706.
- Execution order from here: the rest of E7 (AJ-E706 — which completes AJ-E708 — then the E710 gate) → E10 →
  AJ-E1204/E1205/E1206/E1207 → usage statistics (E11, its own decision
  D-19 / ADR-0015) → E8 tail.
- CI: 5 required checks (`verify`, `ui`, `build-macos`, `build-linux`,
  `build-windows`) under a `main` ruleset (PR required, squash only,
  linear history). The org allows only GitHub-owned or verified
  actions pinned to full SHAs.
- Gates: `just verify`, `pnpm --dir ui test`, `pnpm --dir ui build`,
  and the full Playwright suite for UI changes (696 tests after AJ-E711).
  Local macOS gotchas: cargo needs the Swift shim RUSTFLAGS (see
  CONTRIBUTING.md); the `ports` test binds :5173 and :8000, so stop
  any dev server on those ports before `just verify`.
- Live-machine facts: imported copies of the maintainer's real
  profiles live in the app data dir; originals under other managers'
  roots and `~/.claude` are NEVER touched. Operational tests that read
  the real machine are `#[ignore]`d and name-agnostic.

## Non-negotiable invariants (product + process)

Product (from the original brief, still in force):
- Never automate or read vendor credentials; metadata-only detection,
  vendor's own login in an embedded terminal.
- Automation is passive (monitor/alert/suggest); state-changing
  actions are explicit clicks — with exactly two recorded amendments:
  D-08 cleanup Clean-safe per-Account opt-in, and D-14 pack handling
  per the user's remembered choice. No other exceptions exist.
- Zero writes outside managed directories; pre-existing installs are
  never modified; imports are byte-copies, manifest-verified.
- No traffic proxying/MITM; spawn unmodified real CLIs; all local
  (Fleet: user's own machines only, E7 security model).

Process (this task system):
- No task is DONE without its evidence stored under
  `docs/qa/evidence/<task-id>/` and passing at its stated threshold,
  including its visual-testing artifacts.
- Quality gates are never weakened, skipped or re-thresholded
  silently; that is itself a recorded quality decision (AJ-E806).
- External claims carry confidence labels (Verified-primary /
  Verified-secondary / Negative-evidence / Inferred); negative
  evidence is never worded as a vendor denial (AJ-E810).
- `[inferred]` features (AJ-E1006, AJ-E1008) are falsified by dogfood
  evidence before any build; a recorded REJECTED is a success.

## How to claim and work a task

1. Set its status to `IN-PROGRESS (your-id, YYYY-MM-DD)` in its epic
   file, in your first commit.
2. Work only within its **Files to touch** unless the task is wrong;
   if wrong, fix the task text first (same PR, explained) rather than
   silently expanding scope.
3. Respect every `Depends:` edge; if a dependency is incomplete, mark
   yourself `BLOCKED (reason, AJ-ID)` instead of building on sand.
4. Run the gates the task names (`just verify`, `cargo test
   --workspace`, `pnpm --dir ui build`, visual/a11y suites as they
   land). A red gate is a finding, not an obstacle to route around.
   BATCH them (maintainer directive 2026-10-13, reinforced
   2026-10-15): affected suites only while iterating; and PUSH/CI
   are batched too — MULTIPLE tasks (or a whole phase) per push,
   ONE full chunked pass + ONE PR + ONE CI watch per batch, never
   per task, never per fix. See IMPLEMENTATION-AGENT-PROMPT.md §5
   "Test batching strategy" (level b). If you are about to push and
   the previous push was < a full task ago, STOP — you are
   violating the directive.
5. Store evidence as you go (AJ-E801 manifest), including failures;
   a failed run with analysis beats a curated green one.
6. Flip to `DONE-EVIDENCED` only with evidence links in the same PR.

## Compaction / context-loss recovery

- Your memory of "what we decided" is not evidence. Re-read the
  decision log and this file; check `git log` and task statuses for
  what actually landed.
- If you find a task IN-PROGRESS with no recent commit, treat it as
  unclaimed: read its epic handover note, verify repo state, then
  either claim it fresh or return it to TODO with a note.
- Half-finished documents: the reports in `docs/review/` are frozen
  evidence; corrections belong in new files that cite them, never in
  silent edits (decision log and pass2 §5.2 rule).
- When in doubt about a settled point, grep the decision log first,
  the SPEC second, chat last.

## Traps that have already bitten this project

- Quota-first framing (corrected by D-01): do not re-centre quota.
- One-way "handover" thinking (D-03/D-05): switching is a loop with
  switch-back; copy-back is prohibited.
- Instances/Sessions collapse in shipped UI: Instance is the
  occurrence, Session the conversation inside it (D-02, AJ-E104).
- "Profile" meaning account (other tools' usage): here it is Launch
  settings only (D-10).
- Always-withhold pack redaction was recommended in pass2 and
  superseded by D-14 (user's remembered choice). Check the decision
  log before copying any pass2 "recommended default".
- Fleet deferral in pass2 roadmap superseded by D-13 (launch).
- Scale numbers in early reports are estimates, not measurements;
  AJ-E805 exists to replace them. Never quote them as results.
- Live handover fidelity is unproven on real binaries; AJ-E808 owns
  the truth. Stub/fake results never promote a fidelity badge.
- Credential byte-copy on import contradicts SPEC §8 wording; it is
  an open Phase-0 decision (E9), not a settled fact either way.
- Never touch live user data: `~/.claude`, other managers' profile
  roots, or the running app's state directories. Sacrificial/fixture
  projects only for live-binary validation.

## Open items that are research gates, not decisions

- Provider billing/renewal APIs: unknown until AJ-E601-E605/E909
  record per-Provider proof (D-12). Default product behaviour until
  then: user-entered, optional, never inferred.
- Codex/ZCode/Gemini switch fidelity: UNVALIDATED until AJ-E808.
- Persona claims: composites until AJ-E809 dogfood validates them.

## Who can change what

- Maintainer: decisions (decision log → Final), gate sign-offs,
  quality-gate weakenings, scope-stage approvals.
- Any agent: task execution within scope, evidence, task-text fixes
  with explanation, new follow-up tasks (new AJ IDs, never renumber).
