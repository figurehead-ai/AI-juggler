# E8 — Validation, Visual, Performance & Quality: Implementation Tasks

> Epic owner: cross-cutting proof system. This epic defines HOW every
> other epic proves done. Settled law: D-01 (Switch → See → Clean),
> D-04 (ship in beta, validation as mandatory parallel workstream,
> no pair graduates until its matrix passes), D-07 (measurable signals
> only), D-15 (Stage 1 documents only — these tasks execute under a
> later implementation approval). Do NOT re-litigate D-01..D-15 in
> `docs/review/decision-log.md`. Sources: `docs/SPEC.md:14-24,434-442`,
> `docs/plan/EPICS.md:158-164`, `docs/plan/QUALITY.md:54-65,84-109`,
> `docs/review/pass2/02-unlimited-bidirectional-switching-engine.md`
> §10, `docs/review/pass2/04-daily-indicators-and-right-inspector.md`
> §§3-6, `docs/review/deep/11-ux-redesign-screen-spec.md` §§6-9,
> `docs/review/deep/13-critic-gaps-and-crosschecks.md` K1-K10.
> Vocabulary (D-02/D-10): Provider=HarnessId, Account, Instance=
> SessionRecord+kind, Session=conversation inside an Instance,
> Profile demoted to "Launch settings". Evidence root for every epic:
> `docs/qa/evidence/<task-id>/` (AJ-E801 owns the layout).
> All tasks: Status TODO until the named evidence artifact exists
> and passes at the stated threshold.

### AJ-E801 Evidence store layout and done-proof contract | Status: DONE-EVIDENCED (zcode-primary, 2026-10-13) | Priority P0 | Effort M | Depends: none
**Goal**: Create `docs/qa/evidence/<task-id>/` as the single, linkable home for every task's proof so a gate reviewer never hunts PR threads.
**User story**: As a maintainer signing a milestone gate (EPICS.md:158), I want one directory per task with a manifest of what was run and what passed, so that I can audit any claim in under two minutes.
**Requirements**:
1. The store SHALL contain one directory per task ID (e.g. `docs/qa/evidence/AJ-E401/`) holding `manifest.md`, raw logs, screenshots, and fixture references; no evidence SHALL live only in a PR description or chat log.
2. `manifest.md` SHALL record, for each claim: requirement ID, exact command/fixture/live step, threshold, observed result, PASS/FAIL, commit SHA, date, and reviewer; a missing field SHALL mark the claim `UNPROVEN`.
3. Every other epic's **Evidence/validation proof** SHALL resolve to a path under this store; a task whose evidence path does not exist SHALL NOT be marked DONE regardless of code state.
4. Binary evidence SHALL be content-addressed by name (`<claim>-<sha8>.png|log|json`); re-running a check SHALL add a new file, never silently overwrite a cited artifact.
5. The store SHALL include `_index.md` generated from the manifests (task, gate, verdict, date) and a `_template/manifest.md` new tasks copy verbatim.
6. Secrets, credentials, and unredacted transcripts SHALL NOT be committed; the template SHALL include a redaction checklist (paths hashed, tokens removed) and a guardrail-style assertion that no pack/evidence file contains a credential pattern.
**UI/UX reasoning**: No user surface; reviewer UX follows NN-g visibility of system status — the gate report (QUALITY.md §8 template) links each criterion to its evidence path, so status is inspectable without trust.
**Performance budget**: Index generation over 200 task dirs < 2 s; manifest write < 50 ms; store growth budget < 50 MB per milestone (screenshots compressed, logs trimmed to the failing window plus header).
**Quality gates**: `just verify` green; a link-check test asserts every `docs/qa/gate-M*.md` evidence link resolves inside the store; no new CI job beyond `just verify` (AJ-E807 owns gate verification).
**Evidence/validation proof**: Fixture: create `docs/qa/evidence/AJ-E801/` by running the template against AJ-E401's existing claims; pass = manifest renders all six fields per claim, link-check passes, and a deliberately broken link fails the check. Store the run log in `docs/qa/evidence/AJ-E801/link-check.log`.
**Visual testing**: None (documentation infrastructure). Screenshot proof deferred to AJ-E802; state "no visual surface changed" in the PR.
**Files to touch**: `docs/qa/evidence/_index.md` (new), `docs/qa/evidence/_template/manifest.md` (new), `docs/qa/gate-M0.md`, `docs/qa/gate-M1.md` (link pass only), `docs/plan/QUALITY.md:86`, `docs/plan/EPICS.md:158`.
**Research basis**: `docs/plan/QUALITY.md:86-109` (gate report template); `docs/plan/EPICS.md:160-164` (functional evidence rule); decision-log D-04 (validation debt is tracked, not optional).
**Handover note**: Trap: do not let epics invent their own evidence homes (`docs/qa/<epic>-*.md` sprawl). If a proof cannot be stored without secrets, store the redacted log plus a `REDACTED: <reason>` line — absence with a reason beats a silent gap.

### AJ-E802 Screenshot harness and states gallery | Status: DONE-EVIDENCED (zcode-primary, 2026-10-13) | Priority P0 | Effort L | Depends: AJ-E801
**Goal**: Provide one Playwright harness that captures every screen in every required state at fixed viewports, so visual claims are reproducible by any agent.
**User story**: As a frontend engineer landing a rail or inspector change, I want `pnpm visual:capture` to produce the full states gallery locally, so that I see empty/loading/error/stale variants before review.
**Requirements**:
1. The harness SHALL run the webview UI (`ui/`, Vite + Playwright) against the mocked bridge (`ui/src/lib/bridge.ts` MOCK data) with deterministic fixtures: seeded Instances/Accounts/Projects covering all 9 `SessionStatus` values (`crates/juggler-core/src/model.rs:188`), quota tiers Measured/Computed/Estimated, and honest-empty data.
2. Capture matrix SHALL be: viewports 1440x900, 1280x800, 1024x680 (Tauri `minWidth/minHeight`, `crates/juggler-app/tauri.conf.json`), and 390x844 (collapsed-rail check); themes light + dark; densities Comfortable + Compact (Minimal where the screen has a rail); states populated, empty, loading (skeleton), error (Banner + retained data), stale (`stale - Nm`), and offline/read-only where the screen spec requires them (deep/11 §8 matrix).
3. The gallery SHALL be a single browsable page (`docs/qa/evidence/_gallery/index.html` generated, or `ui` story route) grouping captures by screen → state → viewport → theme, each cell labelled with task ID and fixture name.
4. Capture SHALL be deterministic: animations frozen (`prefers-reduced-motion`), clocks pinned, network mocked; two consecutive runs on the same commit SHALL produce byte-identical PNGs except for an allowlisted timestamp region.
5. The harness SHALL fail loudly when a required state cannot be reached (missing fixture, unmounted screen) — a skipped state SHALL appear in the gallery as `NOT CAPTURED: <reason>`, never omitted.
**UI/UX reasoning**: WCAG 2.2 and deep/11 §8: every screen must render five states; the gallery makes that contract visible. Apple HIG consistency: fixed viewports include the real Tauri minimum (1024x680) so truncation bugs surface before dogfood.
**Performance budget**: Full gallery capture < 120 s on reference Mac; single-screen capture < 10 s; PNG per cell < 500 KB (palette-quantised); harness adds 0 ms to production runtime (dev-only).
**Quality gates**: `just verify` green (Rust untouched); `pnpm build` (tsc + vite) green; harness self-test: capturing the same fixture twice yields 0 differing pixels outside the allowlist.
**Evidence/validation proof**: Command `pnpm visual:capture -- --screen rail` (or the harness's documented equivalent) produces the rail gallery; pass = all required cells present or explicitly `NOT CAPTURED`, determinism check 0 px diff, output stored under `docs/qa/evidence/AJ-E802/`. A missing state without a reason line = FAIL.
**Visual testing**: This task IS the visual harness: deliver captures for WorkflowRail (9 states), Right Inspector Overview, and Onboarding Detect step as the seed set; comparison method defined here (pixel-diff, AJ-E803 thresholds); a11y spot-check: gallery page itself passes axe 0 critical.
**Files to touch**: `ui/package.json` (scripts only), `ui/playwright.config.ts` (new), `ui/tests/visual/` (new), `ui/src/lib/bridge.ts`, `ui/src/components/WorkflowRail.tsx`, `docs/qa/evidence/_gallery/` (generated).
**Research basis**: `docs/SPEC.md:442` (Playwright on webview); deep/11 §8 state matrix (lines 299-320), §12 testing hooks; `crates/juggler-app/tauri.conf.json` (1360x860 default, 1024x680 min); pass2/04 §7 wireframes.
**Handover note**: The repo has no Playwright dependency today (`ui/package.json`: vite/tsc only) — adding it is this task's scope, pinned and lockfile-committed. Trap: never capture against a live Tauri backend for the gallery; mocked-bridge determinism is the point. Live captures belong to dogfood (AJ-E809).

### AJ-E803 Visual regression thresholds and review workflow | Status: TODO | Priority P0 | Effort M | Depends: AJ-E802
**Goal**: Turn screenshots into a pass/fail gate with human review for intentional change.
**User story**: As a reviewer, I want any visual diff over threshold flagged with before/after/diff images so regressions cannot slip in as "looks fine".
**Requirements**:
1. Each gallery cell SHALL be compared pixel-by-pixel against the approved baseline; default threshold SHALL be 0.1% differing pixels and 0 pixels for text-bearing chrome (rail labels, buttons, badges) unless a baseline-update PR explains the change.
2. A failing comparison SHALL produce a triptych (baseline, current, highlighted diff) stored under `docs/qa/evidence/<task-id>/visual/` and linked from the PR; no failure SHALL be dismissed in chat only.
3. Baseline updates SHALL require a dedicated PR section naming the task ID, the intentional change, and maintainer approval; bulk "update all baselines" without per-screen notes SHALL NOT pass review.
4. Colour-only state changes SHALL be treated as failures if the text/icon equivalent is missing (WCAG 1.4.1), even when pixel diff is small.
5. The workflow SHALL run on UI-changing PRs only, complete in <5 min, and post a summary (cells passed/failed/not-captured) as the gate artifact.
**UI/UX reasoning**: NN-g visibility of status + error prevention: reviewers see exactly what changed per state, not a whole-app video. Baselines cover light/dark and Compact/Comfortable because density and theme are where regressions hide (deep/11 §8).
**Performance budget**: Comparison <5 min in CI for full gallery; triptych generation <2 s per failing cell; baseline store <100 MB with PNG quantisation.
**Quality gates**: Harness self-test with an intentionally shifted button MUST fail; reverting the shift MUST pass. `just verify` and `pnpm build` green.
**Evidence/validation proof**: Store self-test triptych + CI summary in `docs/qa/evidence/AJ-E803/`; pass = seeded regression caught, clean tree passes, one real baseline-update PR follows the per-screen note rule.
**Visual testing**: This task defines comparison for all epics; its own proof captures rail + inspector in both themes at 1280x800 and 1024x680.
**Files to touch**: `ui/tests/visual/` (compare config), `docs/qa/evidence/_gallery/`, `docs/plan/QUALITY.md` (visual gate clause), CI workflow files if present.
**Research basis**: deep/11 §8 state matrix; WCAG 2.2 1.4.1; E802 harness contract; decision-log D-04 (validation is mandatory, not optional).
**Handover note**: Trap: raising thresholds to make a red build green is a quality-gate change requiring maintainer approval recorded in the PR, never a silent config tweak.

### AJ-E804 Accessibility verification programme | Status: TODO | Priority P0 | Effort M | Depends: AJ-E802
**Goal**: Make WCAG 2.2 AA a repeatable proof, not a claim, for every shipped screen.
**User story**: As a keyboard/screen-reader user running parallel agents, I want focus order, labels and announcements verified so supervision does not require a mouse or perfect vision.
**Requirements**:
1. Automated axe-core scans SHALL run on every gallery state; pass = 0 critical and 0 serious violations; moderate violations require a filed task ID in the evidence manifest.
2. Keyboard traversals SHALL be recorded for the 6 core flows (launch, switch 1-click, switch wizard, cleanup confirm, onboarding, inspector tab walk): every action reachable, visible focus, no trap, Esc closes dialogs returning focus to the invoker.
3. A manual screen-reader pass (VoiceOver on macOS) SHALL cover rail, inspector and one wizard per release, with the script and findings stored as evidence; status text SHALL use `role="status"`/live regions without stealing focus.
4. Text contrast SHALL be measured for body and faint text in both themes; anything below 4.5:1 (3:1 large text) SHALL fail or be redesigned, with measured ratios recorded.
5. Reduced-motion and 200% zoom states SHALL be captured for rail and inspector; no information SHALL exist only in animation, colour, or hover.
**UI/UX reasoning**: WCAG 2.2 AA (1.4.1, 1.4.3, 2.1.1, 2.4.7, 4.1.3) and Apple HIG accessibility: a supervision tool is used under fatigue and time pressure, where keyboard and announcement failures become task failures.
**Performance budget**: Axe suite <3 min; traversal scripts executable in <15 min per release; zero runtime cost (dev/test only).
**Quality gates**: Release gate fails on any critical/serious axe violation or broken core-flow traversal; findings without task IDs block the gate.
**Evidence/validation proof**: Store axe JSON, traversal recordings/notes and contrast table in `docs/qa/evidence/AJ-E804/`; pass = thresholds above met on the current build, re-run per release candidate.
**Visual testing**: Captures at 200% zoom and reduced-motion for rail/inspector/wizard at 1280x800, both themes, reviewed against baseline rules of AJ-E803.
**Files to touch**: `ui/tests/visual/` (a11y specs), `ui/src/components/WorkflowRail.tsx`, `ui/src/screens/WorkPanels.tsx`, `docs/qa/evidence/AJ-E804/`.
**Research basis**: pass2/04 §3 indicator redundancy rule; deep/11 §9 a11y spec; prior reports' unresolved contrast item (`styles.css` faint text) must be closed with measurements here.
**Handover note**: Do not certify from code reading. The earlier reviews explicitly left contrast/focus unverified; this task exists to replace inference with measurement.

### AJ-E805 Performance budgets and load proof (10 Accounts / 50 Instances) | Status: TODO | Priority P0 | Effort L | Depends: AJ-E801
**Goal**: Prove the app stays responsive at the scale the product promises, with budgets enforced as gates.
**User story**: As a power user with 10 accounts and dozens of live sessions, I want the rail and inspector to stay instant so supervision never lags the work.
**Requirements**:
1. Budgets SHALL be: cold start to interactive <2.5 s on reference Mac; rail render <300 ms at 50 rows; inspector selection-to-paint <200 ms; terminal input echo <50 ms local; switch pack assembly <5 s for 500 turns; orphan detection <10 s after process exit.
2. A load fixture SHALL seed 10 Accounts, 50 Instances (mixed statuses), 20 Projects and 500-turn transcripts, and drive rail scroll, inspector switching, search and cleanup inventory against it in an automated test.
3. Polling SHALL be budgeted: no watcher/poll under 1 s without a named requirement; CPU idle <2% with 10 live Instances, memory <600 MB excluding child processes, each measured and recorded.
4. Regressions >10% on any budget SHALL fail the gate unless a waiver with task ID and expiry is recorded in the evidence manifest.
5. Budgets SHALL be displayed in the gate report as measured values, never as copied targets.
**UI/UX reasoning**: NN-g response-time limits (0.1 s instant, 1 s flow, 10 s attention): supervision UI must feel instantaneous or users return to terminal sprawl. Budgets are therefore acceptance criteria, not aspirations.
**Performance budget**: This task owns the numbers above; test run <10 min; fixture generation <60 s.
**Quality gates**: `just verify` green; load test green on release candidates; waiver register reviewed at each gate.
**Evidence/validation proof**: Store load-test report (machine spec, fixture hash, measured table, PASS/FAIL per budget) in `docs/qa/evidence/AJ-E805/`; pass = all budgets met or waivered with expiry.
**Visual testing**: Capture rail and inspector under load fixture at 1440x900/1024x680, both themes; pass = no layout collapse, skeletons resolve, stale badges appear honestly under throttled refresh.
**Files to touch**: `crates/juggler-supervisor/` (instrumentation points), `crates/juggler-app/` (timing logs), `ui/tests/` (load driver), `docs/plan/QUALITY.md` (budget table).
**Research basis**: pass2/04 §§3-5 refresh/aggregate budgets; pass2/05 orphan ≤10 s; first-report F5.1 scale risks (thread/watcher/SQLite contention) — this task replaces estimates with measurements.
**Handover note**: Earlier scale numbers were reasoned estimates, explicitly not measurements. Never quote them as results; the first run of this fixture is the baseline.

### AJ-E806 Quality gates and CI enforcement | Status: DONE-EVIDENCED (zcode-primary, 2026-10-13) | Priority P0 | Effort M | Depends: AJ-E801
**Goal**: Encode every epic's quality gate in the repo's own verification entry points so no agent can skip them.
**User story**: As a maintainer, I want `just verify` plus the visual/a11y/perf gates to be the single definition of green.
**Requirements**:
1. `just verify` (fmt/clippy/tests) SHALL remain the Rust gate; UI gates SHALL be `pnpm build`, visual compare (AJ-E803), axe (AJ-E804) on UI-changing PRs; each gate SHALL be named identically in QUALITY.md, CI and this epic.
2. A gate SHALL NOT be weakened, skipped or threshold-raised without a PR that states the reason, the task ID and maintainer approval; the change itself is a quality decision recorded in the evidence manifest.
3. Flaky gates SHALL be quarantined with a task ID and a 14-day expiry, never retried silently until green; quarantine list is reviewed at each milestone gate.
4. Every task marked DONE SHALL link its evidence directory from its epic file status line in the same PR that flips the status.
**UI/UX reasoning**: Reviewer UX per NN-g visibility: one green/red vocabulary across local, CI and gate reports; no private definitions of done.
**Performance budget**: Full local gate suite <15 min on reference Mac; CI visual/a11y subset <8 min.
**Quality gates**: This task is the gate system; its own bar is a seeded failing test/visual/a11y case each proving the pipeline goes red.
**Evidence/validation proof**: Store three seeded-failure runs (unit, visual, axe) in `docs/qa/evidence/AJ-E806/`; pass = each turns the correct gate red and removal restores green.
**Visual testing**: Gate summary rendering captured at 1280x800; pass = failed cells link directly to triptychs.
**Files to touch**: `Justfile` or `justfile` if present (verify first), `.github/workflows/` if present, `docs/plan/QUALITY.md`, `ui/package.json` scripts.
**Research basis**: docs/plan/QUALITY.md gates; docs/plan/RUNBOOK.md process; agent-review-prompt offline gates (`just verify`, `cargo test --workspace`, `pnpm --dir ui build`).
**Handover note**: First inspect what CI actually exists today; do not assume a workflow file. If no CI exists, the task delivers the documented local gate script first and names CI creation as a follow-up task.

### AJ-E807 Milestone gate verification and sign-off | Status: TODO | Priority P0 | Effort M | Depends: AJ-E803, AJ-E804, AJ-E805, AJ-E806
**Goal**: Make each milestone gate (M1 onwards) an evidence review, not a status meeting.
**User story**: As a maintainer signing a gate, I want every acceptance criterion traced to stored proof so beta claims are auditable later.
**Requirements**:
1. Each gate report (QUALITY.md §8 template) SHALL list criteria with evidence paths under `docs/qa/evidence/`; a criterion without a resolving path SHALL be recorded FAIL or DEFERRED with a task ID, never PASS.
2. Gate review SHALL include the visual gallery delta, axe summary, performance measured table and unresolved-items register for that milestone.
3. Beta-exit criteria SHALL include: fidelity matrix status per Provider pair (AJ-E808), zero verified-cleanup false kills, A->B->A continuity result, and Fleet security review sign-off (E7).
4. Sign-off SHALL be recorded with date, commit SHA and reviewer in the gate file; unsigned gates block the next milestone's scope.
**UI/UX reasoning**: NN-g visibility for governance: the gate file is the single status surface; links beat attachments and memory.
**Performance budget**: Gate packet generation <60 s from manifests; review meeting target <60 min with pre-read packet.
**Quality gates**: A deliberately unlinked criterion in a rehearsal gate MUST be caught by the link-check from AJ-E801.
**Evidence/validation proof**: First rehearsal gate packet stored in `docs/qa/evidence/AJ-E807/`; pass = link-check green, unsigned state blocks progression in the documented workflow.
**Visual testing**: Gate packet index captured at 1280x800; pass = every criterion row shows verdict + evidence link without horizontal scrolling.
**Files to touch**: `docs/qa/gate-M1.md` and future gate files, `docs/plan/QUALITY.md`, `docs/plan/EPICS.md` gate section.
**Research basis**: EPICS.md:158-164; QUALITY.md §8; decision-log D-04/D-15 (docs acceptance precedes code stages).
**Handover note**: The M1 gate was previously found unsigned while scope accumulated; closing that paperwork is part of this task's first execution.

### AJ-E808 Provider-pair fidelity validation matrix (operations) | Status: TODO | Priority P0 | Effort L | Depends: AJ-E801
**Goal**: Run and maintain the per-Provider-pair validation matrix that turns beta fidelity badges from claims into earned labels.
**User story**: As a user choosing a switch destination, I want the badge on each pair to reflect tests actually run on real binaries, not optimism.
**Requirements**:
1. The matrix SHALL cover, per ordered pair (Claude->Claude other Account, Claude->Codex, Codex->Claude, plus each new Provider on arrival): transcript resume, rules carry, todos, sidechains/subagents, file history, MCP references, cwd/worktree edge cases (dotted and long paths), and switch-back A->B->A.
2. Each cell SHALL be PASS/FAIL/UNVALIDATED with binary version, date, fixture ID and evidence path; UNVALIDATED is the default and SHALL render as `Beta — unvalidated` in product (E2 owns UI, this task owns truth).
3. Live-binary dogfood cells SHALL run on real installed Providers with a sacrificial project; stub/fake-harness results SHALL NOT promote a cell to PASS.
4. A Provider upgrade SHALL reset affected cells to UNVALIDATED until the matrix re-runs; the reset is automatic in the register, not remembered by a person.
5. Beta-exit SHALL require: same-Provider pairs PASS on all cells; cross-Provider pairs PASS or explicitly FAIL-with-user-facing-drop-list.
**UI/UX reasoning**: Honest-fields rule (D-06) applied to trust: badges are text + icon, never colour alone (WCAG 1.4.1); the drop-list wording comes from this register verbatim.
**Performance budget**: Full matrix re-run <1 working day per Provider release, scripted fixture portion <30 min; register update <10 min per cell.
**Quality gates**: No badge promotion without evidence path; register reviewed at every gate (AJ-E807).
**Evidence/validation proof**: Register + per-cell logs/screen recordings under `docs/qa/evidence/AJ-E808/`; pass = register complete for shipped pairs, each PASS traceable to a live run.
**Visual testing**: Capture the in-product fidelity badge and wizard drop-list for one PASS and one UNVALIDATED pair at 1280x800 both themes; pass = wording matches register exactly.
**Files to touch**: `docs/qa/evidence/AJ-E808/` (register), E2 epic (badge consumption), `docs/research/` (per-Provider notes), test fixtures beside adapters.
**Research basis**: pass2/02 §10 matrix; deep/09 handover test plan; decision-log D-04/D-11; SPEC §7 tiers.
**Handover note**: This is the task earlier reports kept deferring as "needs live dogfood". It is now a named, owned workstream; do not let feature work mark it implied-done.

### AJ-E809 Beta dogfood programme | Status: TODO | Priority P1 | Effort M | Depends: AJ-E807, AJ-E808
**Goal**: Recruit and run beta users (including Vibe Kanban/Crystal refugees) through scripted real workflows with recorded outcomes.
**User story**: As a maintainer, I want weekly evidence that real developers complete the Switch -> See -> Clean loop, so beta exit is based on behaviour not downloads.
**Requirements**:
1. Cohorts SHALL cover at least 3 personas from deep/07 (multi-subscription solo, parallel power user, consultant) with >=5 active users for 4 weeks before beta-exit review.
2. Scripted tasks SHALL include quota-wall switch, switch-back after reset, zombie cleanup, cross-Provider labelled-beta switch, and fleet join; completion, time, errors and verbatim friction notes recorded per run.
3. Success thresholds SHALL be: >=80% task completion unaided, A->B->A continuity judged successful by the user in >=80% of runs, zero false cleanup kills, median switch completion <2 min.
4. Findings SHALL be filed as tasks with persona + evidence links within 48 h; `[inferred]` proposals (pass2/07 W5/W6/W11) SHALL NOT be built until dogfood confirms demand.
5. Privacy: no credentials or unredacted transcripts collected; consent and redaction checklist per AJ-E801.
**UI/UX reasoning**: NN-g discount usability: small-n repeated task runs find the friction that analytics cannot; verbatim notes preserve the user's words for copy decisions.
**Performance budget**: Session <45 min per user per week; analysis <2 h per week; tooling is docs + existing evidence store only.
**Quality gates**: Beta-exit review (AJ-E807) cannot pass without this programme's 4-week report.
**Evidence/validation proof**: Weekly reports + run sheets in `docs/qa/evidence/AJ-E809/`; pass = thresholds measured and reported, including failures, with filed follow-up tasks linked.
**Visual testing**: With consent, capture participant screens at switch and cleanup moments (1280x800 typical); pass = captures redacted, labelled by task not by person.
**Files to touch**: `docs/qa/evidence/AJ-E809/`, `docs/plan/RUNBOOK.md` (dogfood cadence), deep/07 personas (validation annotations only).
**Research basis**: deep/07 validation plan; pass2/07 §13 falsification; pass2/12 dogfood programme; decision-log D-04.
**Handover note**: Personas are composites, not interview data; this programme is where they earn or lose their claims. Update persona files only from recorded evidence.

### AJ-E810 External research evidence standard | Status: TODO | Priority P1 | Effort S | Depends: AJ-E801
**Goal**: Set the proof bar for external claims (Provider APIs/SDKs, competitor facts) so research tasks produce reusable, checkable evidence.
**User story**: As an agent picking up a research task cold, I want a standard for what counts as proven so I do not re-derive or over-claim.
**Requirements**:
1. Every external claim SHALL carry: primary source URL, access date, quoted/paraphrased evidence, and a confidence label (Verified-primary / Verified-secondary / Negative-evidence / Inferred); negative-evidence SHALL never be worded as vendor denial.
2. Billing/renewal-date research (E6) SHALL test each Provider's sanctioned API/SDK/docs live where possible; "no API found" requires a documented search log (queries, sources checked) stored as evidence.
3. Competitor or API claims older than 90 days SHALL be marked stale and re-verified before use in a gate or marketing copy.
4. Secrets SHALL never be used to test an API; sacrificial accounts only, with maintainer approval recorded.
**UI/UX reasoning**: Applies honest-fields (D-06) to research: unknown stays unknown in every downstream document and UI string.
**Performance budget**: Standard adds <15 min per research task; re-verification sweep <1 h per release.
**Quality gates**: Gate reviews reject claims lacking confidence labels; stale claims block marketing use.
**Evidence/validation proof**: This file's first application (E6 billing research log) stored under `docs/qa/evidence/AJ-E810/`; pass = log complete per Provider with labels.
**Visual testing**: None; render this epic's register at 100-col, pass = labels and links readable, no table >4 columns.
**Files to touch**: `docs/tasks/E6-quota-cost-and-billing-research.md` (consumption), `docs/research/` (logs), `docs/qa/evidence/AJ-E810/`.
**Research basis**: decision-log D-12 research mandate; second-pass appendix unresolved items (negative-evidence caveats); pass2/06 search-log precedent.
**Handover note**: If a live API test needs a paid account action, stop and record it as a maintainer action; never improvise with a real user account.
