# E10 — Derived Workflows, Safety Nets and Beta Operations: Implementation Tasks

> Epic owner: Product + Core. Status: TODO (all tasks).
> Settled law — do not re-litigate D-01..D-15 in
> `docs/review/decision-log.md`. Sources: pass2/07 W1-W11 derived
> workflows, pass2/06 external pain research, deep/07 personas,
> deep/10 proposal. Items marked `[inferred]` in their sources
> SHALL NOT be built until their falsification task here records
> dogfood or interview evidence (AJ-E809 consumes the results).
> Vocabulary: Provider / Account / Instance / Launch settings.

### AJ-E1001 Per-Instance identity assertion (wrong-account billing guard) | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: —
**Goal**: Prove, before and during launch, that an Instance is actually running under its intended Account, so work is never silently billed to the machine-default account.
**User story**: As a developer with several subscriptions, I want Juggler to warn me immediately if a session is running under the wrong account, before it burns the wrong quota.
**Requirements**:
1. At launch, the Instance SHALL assert its resolved identity (config dir / account probe, metadata only, never credential content) and display Provider + Account + model on the Instance row before the first prompt is sent.
2. If the probed identity differs from the selected Account, launch SHALL pause with an explicit warning naming expected vs detected, with actions: switch selection, re-probe, or cancel; silent continuation SHALL NOT be offered.
3. The assertion SHALL re-run on resume and on switch-back, and its result SHALL be recorded in the session history (who ran where, when).
4. The mechanism SHALL be metadata-only: no credential file content is read, copied or logged; the probe method per Provider SHALL be documented in the adapter contract.
**UI/UX reasoning**: NN-g #5 error prevention + #1 visibility: identity is shown before work starts, not discovered on a bill. Warning uses expected/detected text, never colour alone (WCAG 1.4.1), and is keyboard-actionable.
**Performance budget**: Probe <1 s at launch, 0 ongoing polling cost; assertion adds <100 ms to resume.
**Quality gates**: Fixture with deliberately mismatched config dir MUST block with the warning; matched fixture launches clean; `just verify` green.
**Evidence/validation proof**: Store mismatched/matched fixture runs under `docs/qa/evidence/AJ-E1001/`; pass = mismatch blocked 100% in tests, live dogfood on one real multi-account machine recorded.
**Visual testing**: Capture the identity chip on the Instance row and the mismatch warning at 1280x800 and 1024x680, light/dark; pass = Provider/Account/model readable as text, warning actions focusable in order.
**Files to touch**: `crates/juggler-adapters/` (probe), `crates/juggler-supervisor/` (launch assert), `ui/src/components/WorkflowRail.tsx`, `ui/src/screens/WorkPanels.tsx`.
**Research basis**: pass2/06 §4 (verified wrong-account billing, claude-code issues cited there); SPEC §8 metadata-only rule.
**Handover note**: This is a P0 trust feature promoted from external evidence, not a nice-to-have. If a Provider offers no metadata probe, the Instance SHALL show `Identity unverified` rather than pretending.

### AJ-E1002 Git worktree isolation for parallel Instances | Status: DONE-EVIDENCED | Priority P0 | Effort L | Depends: AJ-E1001
**Goal**: Give each parallel Instance on the same project an isolated worktree/branch by default so accounts never silently overwrite each other's files.
**User story**: As a developer running three accounts on one repo, I want each Instance in its own worktree with a named branch, so parallel work cannot collide silently.
**Requirements**:
1. Launching a second Instance on a project already in use SHALL offer (default on) a dedicated git worktree with a predictable branch name (`juggler/<instance-short-id>`), created only on explicit user confirmation of path.
2. The Project and Inspector views SHALL show per-Instance branch, dirty state and ahead/behind, refreshed on focus and at most every 30 s.
3. Worktree creation/removal SHALL never delete uncommitted work: removal with dirty state requires an explicit typed or checkbox-confirmed action naming what will be discarded.
4. Non-git projects SHALL show isolation as `Unavailable — not a git project` with no fake alternative offered.
5. Switch and switch-back SHALL preserve the worktree binding; the pack manifest records worktree path and branch (E2 consumes).
**UI/UX reasoning**: NN-g #5 error prevention: the dangerous action (shared checkout) becomes the explicit opt-out, not the default. Dirty-state copy names the consequence in plain language (D-09 precedent).
**Performance budget**: Worktree create <5 s for a 1 GB repo; status poll adds <1% CPU; rail render budget of AJ-E805 unaffected at 50 rows.
**Quality gates**: Fixture: two Instances editing the same file concurrently produce zero silent overwrites; dirty-removal guard blocks in tests; `just verify` green.
**Evidence/validation proof**: Store concurrent-edit fixture log + removal-guard test under `docs/qa/evidence/AJ-E1002/`; pass = no overwrite, guard blocks, dogfood on a real repo recorded.
**Visual testing**: Capture branch/dirty chips on rail + Inspector Processes/Overview at 1280x800 both themes; dirty-removal dialog at 1024x680; pass = text states, focus order, no colour-only dirty signal.
**Files to touch**: `crates/juggler-core/` (project/worktree model), `crates/juggler-supervisor/` (launch cwd), `ui/src/screens/WorkPanels.tsx`, `ui/src/components/WorkflowRail.tsx`.
**Research basis**: pass2/06 §7 (verified collision evidence, promoted to P0); first-report F1.1; SPEC pack branch field.
**Handover note**: Earlier reports marked collisions `[inferred]`; pass2/06 upgraded the evidence. Do not downgrade without reading that section.

### AJ-E1003 Quota reset queue and parked continuation | Status: DONE-EVIDENCED | Priority P1 | Effort M | Depends: AJ-E1001
**Goal**: Let a quota-blocked Instance be parked with its context pack staged, and surface it for one-click continuation the moment its window resets.
**User story**: As a developer blocked at 11pm, I want my work parked safely and offered back to me at reset, on the original account, without reconstructing anything.
**Requirements**:
1. A QuotaBlocked Instance SHALL offer "Park until reset" which stages the portable context pack (E2), records the reset time from the quota engine (E6), and moves the Instance to a visible Parked state.
2. At reset, Juggler SHALL notify (never auto-launch) and offer one-click resume-original per D-05; dismissal snoozes once, then leaves the Instance in Parked without repeated nagging.
3. Parked Instances SHALL appear in the rail and Inspector with reset countdown as text, sortable by reset time in morning triage (AJ-E1005).
4. If quota data is Estimated rather than Measured, the copy SHALL say so and offer "check now" instead of a false countdown.
**UI/UX reasoning**: NN-g #1 visibility + anti-fatigue notification design: one notification at the actionable moment, snooze once, no badge spam. Countdown is text, not only a progress ring (WCAG 1.4.1).
**Performance budget**: Park action <2 s including pack staging for 500 turns; reset watcher is part of the E6 quota poll, no extra timer under 60 s.
**Quality gates**: Fixture with simulated reset fires exactly one notification; Estimated-quota fixture shows no countdown; `just verify` green.
**Evidence/validation proof**: Store simulated-reset run + notification log in `docs/qa/evidence/AJ-E1003/`; pass = single notification, correct resume target, dogfood overnight run recorded (W1).
**Visual testing**: Capture Parked row, reset notification and resume offer at 1280x800 light/dark; pass = countdown text, one-click action keyboard reachable, dismiss/snooze distinct.
**Files to touch**: `crates/juggler-core/` (parked state), `crates/juggler-app/` (notification), `ui/src/components/WorkflowRail.tsx`, E2/E6 interfaces.
**Research basis**: pass2/07 W1/W3; deep/10 reset-queue proposal; decision-log D-05.
**Handover note**: Passive invariant: notification + user click only. Auto-resume on reset is explicitly rejected scope; do not add it as a convenience.

### AJ-E1004 Overnight and unattended run supervision | Status: DONE-EVIDENCED | Priority P1 | Effort M | Depends: AJ-E1003
**Goal**: Make long unattended runs observable and safely interruptible without babysitting a terminal.
**User story**: As a developer starting an evening run, I want a morning summary of what finished, what got stuck and what it cost, instead of scrolling a dead terminal.
**Requirements**:
1. Instances SHALL emit needs-attention states (Stuck, WaitingUser, QuotaBlocked, Crashed) into the rail/Inspector; an Instance with no output for a Provider-configured threshold SHALL be flagged Stuck-candidate with the threshold shown.
2. A morning summary SHALL aggregate overnight Instances: finished, needs attention, errors, tokens/cost where measurable, and leftover processes (E5 inventory), each row linking to its Inspector.
3. Notifications SHALL be batched into the summary by default; only Crashed and wrong-account events (AJ-E1001) MAY interrupt immediately, per Account notification settings.
4. Every state transition SHALL be timestamped in session history so the summary is auditable, not reconstructed from memory.
**UI/UX reasoning**: NN-g #1 visibility at a glance: one summary surface, severity ordered, honest Unavailable for unmeasured cost (D-06/D-07). No composite health score (D-07).
**Performance budget**: Summary generation <2 s for 50 Instances; state evaluation piggybacks existing supervisor events, no new poll under 5 s.
**Quality gates**: Fixture Instances in each state produce the correct summary rows; no transition is lost across an app restart (journal replay test).
**Evidence/validation proof**: Store fixture summary + restart-replay log in `docs/qa/evidence/AJ-E1004/`; pass = all states represented, dogfood overnight run (with AJ-E1003) recorded.
**Visual testing**: Capture morning summary and rail attention badges at 1440x900/1024x680 both themes; pass = severity readable as text+icon, summary rows keyboard navigable.
**Files to touch**: `crates/juggler-supervisor/` (state emission), `crates/juggler-core/` (history), `ui/src/screens/` (summary surface), `ui/src/components/WorkflowRail.tsx`.
**Research basis**: pass2/07 W1; first-report F2.5/F5.2 (states modelled but never emitted — this task closes that at the workflow layer; E4 owns the read-model).
**Handover note**: Coordinate state emission with E4/E5 owners to avoid two competing state machines; supervisor journal is the single source.

### AJ-E1005 Morning triage and multi-project day board | Status: DONE-EVIDENCED | Priority P1 | Effort M | Depends: AJ-E1004
**Goal**: Give the start of day one sorted view: what needs me, what is parked, what is running, across all projects.
**User story**: As a consultant with five client projects, I want to triage fifteen sessions in two minutes each morning, oldest blocker first.
**Requirements**:
1. A triage view SHALL list Instances grouped by needs-attention, parked-by-reset, running, and finished-unreviewed, sortable by waiting time, project and quota risk.
2. Each row SHALL carry the E4 indicator set and one primary action (open, resume, switch, clean, archive) without opening a second screen.
3. Triage SHALL be fully keyboard operable (j/k or arrows, Enter opens, action shortcuts shown) and completable for 15 Instances in a timed usability script.
4. Finished-unreviewed Instances SHALL leave the list via an explicit reviewed/archive action recorded in history (AJ-E1007 lifecycle).
**UI/UX reasoning**: NN-g #6 recognition over recall + Fitts/Hick at scale: grouping by required action, not by tool, matches the user's morning question ("who needs me?").
**Performance budget**: Triage render <300 ms at 50 Instances (AJ-E805 budget); action round-trip <200 ms to visible state change.
**Quality gates**: Timed script: 15-Instance triage <2 min by a dogfood user; no action requires a mouse; `just verify` green.
**Evidence/validation proof**: Store timed triage run (AJ-E809 cohort) in `docs/qa/evidence/AJ-E1005/`; pass = time + completion thresholds met, friction notes filed as tasks.
**Visual testing**: Capture triage groups populated/empty at 1440x900/1024x680 both themes and densities; pass = group meaning text-labelled, primary actions visible without hover.
**Files to touch**: `ui/src/screens/` (triage view), `ui/src/components/WorkflowRail.tsx`, E4 read-model.
**Research basis**: pass2/07 W3/W4; deep/07 consultant persona; E4 indicators.
**Handover note**: This is a view over E4 data, not a second data model. If E4 fields are missing, triage shows Unavailable per D-06 rather than inventing layout filler.

### AJ-E1006 Pair review and model arbitrage workflows [inferred — falsify first] | Status: REJECTED (D-26, 2026-10-20) | Priority P2 | Effort M | Depends: AJ-E1005
**Goal**: Test whether users actually want author-on-one-account/review-on-another and cheap-model-drafts/strong-model-finishes as built workflows, before building them.
**User story**: As a power user, I want a second account to review what the first wrote, and cheap exploration before expensive execution — if these prove to be real habits.
**Requirements**:
1. This task SHALL first run a falsification round (>=5 dogfood users, AJ-E809): diary + interview evidence for or against each workflow, recorded with persona and verbatim notes.
2. Only a workflow with >=3 independent users requesting it SHALL graduate to a build task (new AJ ID) specifying pack-based handoff into a review Instance with diff-first Inspector view.
3. If evidence is negative, this task SHALL close as REJECTED with the evidence linked, and the roadmap SHALL NOT carry the feature as planned.
4. Any graduated design SHALL respect D-03/D-05 (review is a switch/fork with lineage, never a silent copy).
**UI/UX reasoning**: Evidence-before-UI: NN-g user research precedes surface design; no screen is specified until behaviour is observed.
**Performance budget**: Research round <=3 weeks elapsed, <=2 h per participant; no product performance surface.
**Quality gates**: Graduation or rejection both require the evidence packet; an undecided outcome keeps the item out of beta scope.
**Evidence/validation proof**: Store interview/diary packet in `docs/qa/evidence/AJ-E1006/`; pass = explicit graduate/reject decision recorded in the decision log as a new entry.
**Visual testing**: None until graduation; if graduated, the build task SHALL define captures per AJ-E802/E803 before code.
**Files to touch**: `docs/review/decision-log.md` (new decision entry on outcome), `docs/qa/evidence/AJ-E1006/`, pass2/07 annotations.
**Research basis**: pass2/07 W5/W6 marked `[inferred]`; second-pass critic U-items (no competitor evidence); decision-log process rule.
**Handover note**: Do not let enthusiasm substitute for evidence — the reports explicitly flagged these as unvalidated. A clean REJECTED is a successful outcome for this task.
**Outcome (2026-10-20, D-26)**: REJECTED as built workflows. A desk falsification (`docs/qa/evidence/AJ-E1006/desk-falsification.md`) found the step each one says is missing already served. W6: Claude Code's `opusplan` and `/model` within one account, and the app's Switch account… across accounts. W5: `/review`, `codex review` and OpenAI's `/codex:review` plugin for Claude Code, reached in the app by New session on another account in the same project. The user round in `protocol.md` didn't run (no AJ-E809 participants yet). It stays the way back: 3 or more participants showing the habit and friction with these paths open a new task.

### AJ-E1007 Session lifecycle: search, archive, replay, retention | Status: DONE-EVIDENCED | Priority P1 | Effort M | Depends: AJ-E1005
**Goal**: Give sessions a full lifecycle so months of work stay findable and the rail stays usable.
**User story**: As a long-term user, I want to search, archive and replay any past session in seconds, and to control how long transcripts are kept.
**Requirements**:
1. Global search SHALL cover project, account, provider, model, branch, date and transcript text (local index), returning results in <500 ms over 1,000 sessions with the matching context snippet shown.
2. Archive SHALL remove an Instance from active views without deleting data; unarchive restores it exactly; delete is a separate, confirmed, irreversible action naming the data destroyed.
3. Replay SHALL open any archived/ended session read-only in the terminal view with its history, lineage and process record attached.
4. Retention SHALL be user-configured per Account (keep forever default), with a settings display of storage used per Account and a dry-run preview before any bulk cleanup.
**UI/UX reasoning**: NN-g #3 user control + #6 recognition: destructive actions are separated, previewed and named; search snippets prevent open-every-result hunting. All actions keyboard reachable with visible focus (AJ-E804 flows).
**Performance budget**: Index build <30 s per 1,000 sessions incremental thereafter; search <500 ms; archive/unarchive <1 s; storage accounting <2 s.
**Quality gates**: Fixture: archived session replays byte-identical transcript; delete-confirm test proves no single-click path exists; index correctness 100% on fixture set.
**Evidence/validation proof**: Store search/archive/replay fixture runs in `docs/qa/evidence/AJ-E1007/`; pass = thresholds met, dogfood user finds a 3-week-old session unaided (AJ-E809 script).
**Visual testing**: Capture search results, archived list, replay view and retention settings at 1280x800/1024x680 both themes; pass = state text (Archived/Replay read-only) explicit, delete dialog consequence copy exact.
**Files to touch**: `crates/juggler-core/` (index, lifecycle states), `ui/src/screens/` (search/archive/replay), settings surface (E3/E4 hooks).
**Research basis**: first-report F1.3; deep/10 session lifecycle; pass2/07 W10 cost-review dependency.
**Handover note**: Retention cleanup interacts with E5's never-touch-unowned rule: retention deletes Juggler-managed data only, listed by the dry-run, never by age alone without user confirmation.

### AJ-E1008 Provider outage failover day [inferred — falsify first] | Status: IN-PROGRESS | Priority P2 | Effort M | Depends: AJ-E1006
**Goal**: Determine, from observed outage behaviour, whether whole-project failover to another Provider needs a built flow or is already served by switching.
**User story**: As a developer whose provider is down, I want to move a whole project to a working provider without re-explaining context per session.
**Requirements**:
1. This task SHALL first log real outage events during beta (detection source, duration, user workaround) via AJ-E809 reporting; no detection feature is built on speculation.
2. If >=3 outage events show project-level pain, a build task SHALL be created: project-level switch offer reusing E2 pack/switch machinery per Instance, with per-Instance fidelity badges (D-11).
3. Juggler SHALL NOT auto-detect "outage" from a single failed request; any future detector requires a documented signal source and false-positive analysis.
4. Until graduation, the supported path (manual per-Instance switch) SHALL be documented in help copy with honest steps.
**UI/UX reasoning**: Error-prevention over false automation: a wrong outage declaration that migrates work is worse than a manual switch (NN-g #5).
**Performance budget**: Logging adds 0 runtime cost; any graduated design inherits E2 switch budgets (<5 s pack, <=2 clicks).
**Quality gates**: Graduate/reject decision recorded with event evidence; no detector ships without false-positive analysis.
**Evidence/validation proof**: Outage event log in `docs/qa/evidence/AJ-E1008/`; pass = decision (build with new task ID, or reject) recorded in decision log.
**Visual testing**: None until graduation; help-copy state captured with AJ-E803 gallery if text ships.
**Files to touch**: `docs/qa/evidence/AJ-E1008/`, `docs/review/decision-log.md` (outcome entry), E2 (consumed machinery).
**Research basis**: pass2/07 W11 `[inferred]`; critic note requiring outage logging before building.
**Handover note**: Same rule as AJ-E1006: an evidence-based REJECTED closes this task successfully. Do not build monitoring infrastructure for a hypothetical.

### AJ-E1009 Backup, restore and machine migration of Juggler's own data | Status: DONE-EVIDENCED | Priority P1 | Effort M | Depends: AJ-E1007
**Goal**: Let users export and restore Juggler's database, packs and settings so a new machine or a corrupt store never costs them their history.
**User story**: As a user moving laptops, I want one export file I can carry over and restore, with proof the restore matches.
**Requirements**:
1. Export SHALL produce a single archive (DB snapshot, packs, settings, lineage) with a manifest and checksums; credentials, provider login material, and the Fleet `fleet/` folder (machine identity keys and peer pins — AJ-E701 contract) SHALL be excluded by construction and the exclusion verified by the AJ-E801 redaction check and a test.
2. Restore SHALL verify checksums and schema version, refuse incompatible versions with a named reason, and restore into a clean managed directory without touching pre-existing vendor installs (invariant).
3. A restore rehearsal SHALL run in CI on a fixture archive: restore, launch read-model, assert counts and lineage integrity match the manifest.
4. Fleet machines SHALL each keep independent backups; restoring one machine's backup onto another SHALL warn that it carries that machine's data, never its identity: the restored Mac keeps its own identity key and must pair again (E7 identity).
**UI/UX reasoning**: NN-g #5/#9: destructive-adjacent flows state scope, verify first, and report in plain language; progress and completion are visible, never silent (visibility of status).
**Performance budget**: Export <60 s for 1 GB store (streaming, progress shown); restore verification <30 s; no background upload anywhere (local file only).
**Quality gates**: Fixture restore test green in `just verify`; credential-pattern scan of a real export structure passes (patterns only, no real secrets).
**Evidence/validation proof**: Store rehearsal log + manifest diff in `docs/qa/evidence/AJ-E1009/`; pass = counts/checksums match, dogfood migration of one real (consented, redacted) store recorded.
**Visual testing**: Capture export progress, restore verify result and machine-identity warning at 1280x800 both themes; pass = outcome text explicit, failure states name the file and reason.
**Files to touch**: `crates/juggler-core/` (export/import), `crates/juggler-app/` (commands), settings UI, E7 identity model.
**Research basis**: first-report F1.7; deep/10 backup proposal; SPEC §12 storage.
**Handover note**: Export format versioning is an ADR-worthy decision if the schema is non-trivial; raise it rather than inventing a silent format.

### AJ-E1010 Updates and release integrity for beta | Status: IN-PROGRESS (every requirement evidenced in tests, by hand and on AI Juggler Dev, 2026-10-20; the signed-build demonstration is the maintainer's) | Priority P1 | Effort M | Depends: AJ-E1009
**Goal**: Ship beta updates that are signed, announced and safely reversible, so early users trust the update path.
**User story**: As a beta user, I want to know what changed, update in one click, and roll back if a build breaks my workflow.
**Requirements**:
1. Releases SHALL be signed artefacts with a changelog entry per release; the app SHALL check for updates on launch (user-disableable) and show version + changes before installing, never auto-install silently.
2. A rollback path to the previous version SHALL be documented and rehearsed once per release channel; user data migrations SHALL be forward-compatible or ship with a downgrade note.
3. Beta channel SHALL be labelled as beta in-app (About + update prompt) consistent with D-04/D-11 labelling.
4. Failed update (checksum/signature/network) SHALL leave the running version untouched and report the reason with a retry action.
**UI/UX reasoning**: NN-g #1/#3: update state, progress and undo are visible and user-initiated; no surprise restarts during live sessions — prompts defer until no Instance is mid-switch.
**Performance budget**: Update check <1 s background, 0 launch blocking; download progress at least every 5%; check disabled = zero network calls (verified).
**Quality gates**: Signature-failure fixture refuses install; rollback rehearsal recorded per release; migration fixture (previous DB opens or downgrade note exists).
**Evidence/validation proof**: Store signed-artefact verification log + rollback rehearsal in `docs/qa/evidence/AJ-E1010/`; pass = all four requirements demonstrated on a real beta build.
**Visual testing**: Capture update-available, downloading, failed and up-to-date states at 1280x800 both themes; pass = version/changes visible before install, beta label present, no modal trap (AJ-E804).
**Files to touch**: release pipeline (inspect current state first), `crates/juggler-app/` (updater), About/settings UI, `CHANGELOG.md` process.
**Research basis**: first-report release gaps (M5 unsigned releases placeholder); pass2/12 GTM/beta; SPEC § release notes — verify current state in repo before assuming.
**Handover note**: Inspect what release machinery exists today before designing; earlier docs said signed releases were not built. Do not design against an assumed pipeline.

### AJ-E1011 Team pack export and colleague handoff | Status: TODO | Priority P2 | Effort M | Depends: AJ-E1007, AJ-E1009
**Goal**: Let a user hand a project context pack to a colleague, who can open it in their own Juggler under their own accounts.
**User story**: As a consultant handing over a client project, I want to export the context so my colleague continues without my accounts, my credentials or my machine.
**Requirements**:
1. Team export SHALL reuse the E2 pack format with the D-14 redaction policy of the exporting user applied and shown; account identities in the pack SHALL be descriptive labels only, never credentials or config-dir paths from the exporter.
2. Import SHALL validate the pack, map its contents to the importer's own Provider/Account choices via the onboarding inference UI (E3), and record lineage marking the cross-user origin.
3. Packs containing withheld items SHALL list them so the recipient knows what to re-supply (their own keys/rules) instead of discovering gaps mid-task.
4. Fleet and team export SHALL remain distinct concepts in UI copy (Fleet = my machines; Team export = a file for another person).
**UI/UX reasoning**: NN-g #2 real-world match: export/import language mirrors file sharing users know; the withheld-items list prevents silent context holes (honest-fields D-06).
**Performance budget**: Export/import inherit E2 pack budgets (<5 s/500 turns); validation <2 s before any write.
**Quality gates**: Fixture: pack exported on fixture machine A opens on fixture B with zero exporter paths/credentials present (automated scan); lineage origin recorded.
**Evidence/validation proof**: Store cross-fixture scan + import run in `docs/qa/evidence/AJ-E1011/`; pass = zero leakage hits, dogfood handoff between two real users recorded if cohort allows.
**Visual testing**: Capture export summary (with withheld list) and import mapping screen at 1280x800 both themes; pass = origin, policy and withheld items readable as text.
**Files to touch**: E2 pack module, E3 import flow, `crates/juggler-core/` (lineage origin), share/import UI.
**Research basis**: pass2/07 W9; deep/10 pack sharing signal (templates demotion revisit trigger); D-14.
**Handover note**: Demand signal comes from AJ-E809 dogfood (repeated pack sharing). If absent by beta-exit, keep this P2 and do not promote on enthusiasm.

### AJ-E1012 Settings — the policies the user owns (wireframe 26) | Status: DONE-EVIDENCED | Priority P1 | Effort M | Depends: AJ-E208, AJ-E508, AJ-E610, AJ-E1203
**Goal**: One home for every "whose choice is it?" setting, visible and reversible, as approved in wireframe 26.
**User story**: As a user, I want to see and change every policy AI Juggler follows for me in one place, without hunting through screens or Inspectors.
**Requirements**:
1. Settings SHALL show a section list beside the settings — General, Account & Cloud, Context packs & secrets, Cleanup, Accounts & renewal dates, Notifications, Fleet permissions, Data — that jumps to a section, moves focus to its heading, and marks where you are; every section stays on the page so search finds any setting.
2. Context packs & secrets SHALL reuse the D-14 global + per-Provider control (the same component as Accounts).
3. Cleanup SHALL list each Account's two D-08 toggles and offer a default for new Accounts that is "Notify only" unless the user picks otherwise; the default SHALL reach only Accounts created after the choice, and never Tier 2.
4. Accounts & renewal dates SHALL show each Account's user-entered date (D-12; "Not set" means no reminders) and the reminder lead time (1–30 days, default 7).
5. Notifications SHALL let the user set the digest's quiet hours (default 22:00–07:00; equal hours = none; deferred, never dropped) and choose which Accounts' lines join the hourly digest; actionable alerts stay at-most-once per window.
6. Fleet permissions SHALL list paired Macs and change access through the same sheet as the Fleet screen.
**Quality gates**: Unit + integration tests for quiet hours, digest mutes and the cleanup default; vitest for every section; axe 0 on the new gallery states; core visual baselines re-rendered with notes.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1012/`.
**Visual testing**: Gallery `65-settings-sections` (each new section at 1280×800 and the list at 1024×680, light/dark).
**Research basis**: wireframe 26 (approved 2026-10-13); D-08, D-12, D-14; AJ-E611 anti-fatigue rules.

### AJ-E1013 First run, rebuilt on the UX vision (D-25) | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E301, AJ-E307, AJ-E308, AJ-E1012
**Goal**: A fresh install opens on what's already on this computer and gets the user working in three calm steps.
**User story**: As a developer with Claude Code set up in the terminal and in the Claude desktop app, I want AI Juggler to find all of it, bring it in with one click, help me sign each copy in, and set up what the OS needs — without reading a manual.
**Requirements**:
1. With no Account, first run SHALL open full-window on the scan's results, grouped as the user thinks of them (Claude desktop app profiles, Claude Code in the terminal, other setups), preselected (signed in or used in 30 days; never one already in AI Juggler), with one "Bring in N accounts" button; imports are copies and say so. "Where we looked" SHALL show every probe.
2. Each copy that still needs the vendor's sign-in SHALL be one row (name, status, Sign in; the rest behind More), polled every 2 s, with "Sign in to all, one after another" that moves on by itself.
3. Computer access SHALL list what this platform reports (D-25) with one line of why, live status, Allow (system prompt) or Open System Settings once declined; it is polled, and skipped when the OS needs nothing.
4. The pack-secrets question SHALL NOT be asked in first run; the first switch asks while no choice exists (D-14).
**Quality gates**: Backend tests for the access list (never prompting in tests); vitest for the rules and the flow; the journey `journey-first-run.spec.ts` end to end; axe 0 on its captures.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1013/`.
**Visual testing**: Gallery `66-first-run` (what's here, where we looked, sign-in, access, after).
**Research basis**: D-25 and the "AI Juggler UX vision" canvas; maintainer report 2026-10-19 (first run opened on a CLI check and missed the Claude desktop profiles).


### AJ-E1014 Home and the one rail (D-25) | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E1013, AJ-E403, AJ-E404
**Goal**: Opening the app answers "what needs me, what's running, how much runway is left" at a glance; everything else is one step away.
**User story**: As a developer juggling several sessions on several subscriptions, I want one calm list of my projects and their sessions and a Home that tells me what to do next, not every field the app knows.
**Requirements**:
1. The top level SHALL be Home, Accounts, History and Settings; Triage, Overview, Usage, Fleet and Cleanup sit under More. Cleanup SHALL show as its own pill while it has alerts, Fleet once a peer is paired. The More menu closes on Escape or a click elsewhere and paints above the page.
2. The rail SHALL be one list grouped by project (a project header opens the project), with search, the quick filters Needs you · Running · All, and a Filters button that holds the rest (Ended, sort, group, provider, account, machine) and says when any is on.
3. A row SHALL show the session's state as an icon and words, its branch or folder, how long, and its account's colour; it SHALL add only the exceptions worth a glance — quota at 25 % or less, parked, uncommitted files, a sign-in to check, a value that stopped updating, a desktop-app session.
4. Home SHALL show Needs you, Running, Runway (the tightest measured window per account; "Not measured yet" otherwise) and Pick up where you left off; with no accounts it is one plain next step.
5. Density SHALL move to Settings › Appearance, with each choice described; Comfortable is two lines per session, Compact one.
6. The session view SHALL lead with a short summary (state, account, model, branch, park) and keep the rest under Details; announcements and source footers use plain words, not rule codes.
**Quality gates**: vitest for the rail, Home and the inspector; the full Playwright suite; axe 0 on the captures; visual baselines re-rendered with notes.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1014/`.
**Visual testing**: Gallery `01-rail`, `01-rail-density`, `01-rail-sort`, `05-session-panel`, `06-inspector`, `67-home`.
**Research basis**: D-25 and the "AI Juggler UX vision" canvas; maintainer report 2026-10-19 (Usage blank, densities identical, two similar rail tabs, inline legend, busy filters).

### AJ-E1015 The screen says "session" (D-25) | Status: DONE-EVIDENCED | Priority P1 | Effort S | Depends: AJ-E1014
**Goal**: One word for the thing the user juggles, everywhere they read it.
**User story**: As a user, I want the app to call a running conversation a session — the word I use — not an internal noun.
**Requirements**:
1. Every on-screen string, visible attribute (label, title, placeholder, alt) and backend message that reaches the screen SHALL say "session" where it said "Instance"; a Session (the conversation record) SHALL show as "Conversation".
2. Code, SQL, metric names and internal docs SHALL keep their names (D-25).
3. The terminology lint SHALL refuse "Instance" in JSX text and visible attributes.
4. README and the user guide SHALL follow.
**Quality gates**: vitest, Rust tests, the full Playwright suite, the lint (with a seeded regression).
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1015/`.
**Research basis**: D-25 ("use 'session' as the on-screen word").

### AJ-E1016 Accounts with runway, a Usage screen that explains itself, the limit decision (D-25) | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E1014, AJ-E610, AJ-E1003, AJ-E206
**Goal**: "How much do I have left, and what do I do when it runs out?" is answered where the user looks, in words.
**User story**: As a developer on several subscriptions, I want each account's runway at a glance, the best account for new work named, and one calm decision when an account hits its limit.
**Requirements**:
1. Each account on Accounts SHALL show its 5-hour and weekly windows in words (% left, when it resets; "Used up until …", "Measuring", "Not measured yet"), the pace line only when it will run out, and what runs on it; with no account reading, the newest session's own quota line, named as such.
2. Accounts SHALL name the best account for new work (signed in, most left in its tighter window). Sign-in shows only when it is needed. A disabled Start says why.
3. The pack-secrets policy SHALL be one link away (Settings › Context packs & secrets, opened at that section); the Beta matrix SHALL sit behind a disclosure.
4. Usage SHALL lead with every account's runway, then what sessions used; with no accounts it says what will appear and offers Find my accounts — never a blank page.
5. When a session is stopped by its limit, Home's "Continue elsewhere" SHALL open one decision: Continue now on the ranker's pick (its runway, what carries over), Wait for the reset (park; offered back at the reset; nothing restarts on its own), or Choose another. It is a modal dialog with focus inside, Escape closes it, and nothing happens until a choice.
6. Accounts › Add account SHALL open first run's find → bring in → sign in (closable; straight to a fresh account when nothing new is found; no access step when all is allowed).
**Quality gates**: vitest (`account-runway.test.tsx`, `limit-dialog.test.tsx`); `accounts-gallery.spec.ts` with axe; the full Playwright suite; baselines re-rendered with notes.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1016/`.
**Visual testing**: Gallery `68-accounts` (Accounts full and signed out, Usage full and with no accounts, the limit decision; both themes).
**Research basis**: D-25 and the "AI Juggler UX vision" canvas (Limit board); maintainer report 2026-10-19 (Usage blank).

### AJ-E1017 The session view leads with where and what (D-25) | Status: DONE-EVIDENCED | Priority P1 | Effort S | Depends: AJ-E1014, AJ-E209
**Goal**: Opening a session says what it is and where, with the next move beside it; the ranked switch list is one step away.
**User story**: As a developer with several sessions on one branch name or account, I want the session I opened named by where it runs, not by its account, and the switch list there when I want it — not on every visit.
**Requirements**:
1. The session header SHALL name the session by its place (branch, else folder) with its state, and a line with project · account · started · runway when low; the top bar SHALL name the project.
2. Switch account… SHALL sit beside Stop. The ranked list SHALL fold behind "Continue on another account": open by itself when the session hit its limit or a way back is on offer; Switch account… and ⌘⇧H open it and move focus to it.
3. The summary card SHALL not repeat the state or account the header shows.
4. The six inspector tabs stay as D-06 fixes them.
**Quality gates**: vitest (`inspector-shell.test.tsx`); the full Playwright suite (keyboard flows reach the list with ⌘⇧H); baselines re-rendered with notes.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1017/`.
**Research basis**: D-25 and the "AI Juggler UX vision" canvas (Session board).

### AJ-E1018 0.0.6 dogfood fixes: the legend, where setups are open, sign-ins on Home | Status: DONE-EVIDENCED | Priority P0 | Effort S | Depends: AJ-E1013, AJ-E1014
**Goal**: Fix what the maintainer hit in the first minutes of using 0.0.6.
**User story**: As the maintainer trying the beta, I want the "?" to show me what the icons mean, first run to see my six Claude desktop windows for what they are, and Home to tell me when an account needs its sign-in.
**Requirements**:
1. The rail's "?" SHALL open a legend that is fully in view and on top, matches the rail's words (no internal codes), and closes on Escape or a click elsewhere (focus returns to "?").
2. The scan SHALL report, for each setup, where it is open right now (the Claude desktop app and/or a terminal), read from the running Claude processes (only `CLAUDE_CONFIG_DIR` is read); first run SHALL group by kind of setup (profiles / the default / others), name the default "Default" (not "Terminal"), and show where each is open. Probe labels SHALL be plain words.
3. An account that needs its sign-in SHALL be a "Needs you" item on Home whose Sign in opens the login terminal and clears by itself when the sign-in completes; the old "Get your accounts ready" screen is retired.
**Quality gates**: vitest; Rust tests (the running-setups classifier); the full Playwright suite; the accessibility gate; visual baselines re-rendered with notes; walked by hand in the browser.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1018/`.
**Research basis**: maintainer reports 2026-10-20 (screenshots of first run and the rail; "clicking on the question mark … does nothing").

### AJ-E1019 Cleanup in plain words; "tool" on screen instead of "Provider" (D-25) | Status: DONE-EVIDENCED | Priority P1 | Effort S | Depends: AJ-E506, AJ-E508, AJ-E1015
**Goal**: Cleanup says what it is in a sentence, keeps its switches one step away, and no screen uses the internal word "Provider".
**User story**: As a developer who opens Cleanup because a pill said something was left running, I want to see what was left and what I can do — not "Tier 0 observe · verified manual kills".
**Requirements**:
1. The screen SHALL be titled "Cleanup" with one plain line (what was left running; nothing is stopped unless chosen or turned on). No tier names on screen; the tiers stay in Settings › Cleanup.
2. Pause automatic cleanup SHALL sit behind the header's ⋯ (with Cleanup settings…, opening Settings at its Cleanup section); while paused, the banner carries Resume. The kill-switch stays in the header (AJ-E508 Req.4).
3. The machine filter SHALL show only once another Mac is paired or reporting (AJ-E511's default stays This machine).
4. Groups SHALL name the tool ("Claude Code"), not its id; the backend's alerts say "review it in Cleanup".
5. On screen, "Provider" SHALL read "tool" (History's search line, Settings, pack secrets, the rail's Filters, the session details, backup's list of what isn't included, quota's "no verified source"). The code keeps `Provider`. `scripts/terminology_lint.sh` SHALL refuse "Provider" in JSX text, visible attributes and user-facing strings.
**Quality gates**: vitest (`cleanup-centre.test.tsx`: title and no tier words, ⋯ menu, Resume on the banner, no machine filter without another Mac); terminology lint; the full Playwright suite; baselines re-rendered with notes.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1019/`.
**Research basis**: D-25 (plain words; depth on request; calm until it matters).

### AJ-E1020 The rail folds to icons in a narrow window (D-25) | Status: DONE-EVIDENCED | Priority P1 | Effort S | Depends: AJ-E403, AJ-E1014
**Goal**: Below 900 px the rail stops crowding the screen by itself and comes back when the window widens.
**User story**: As a developer with AI Juggler in a narrow side window, I want the session list to fold to its icon column on its own, without changing my density setting.
**Requirements**:
1. Below 900 px the rail SHALL show its minimal (icon) density; at 900 px and wider, the density chosen in Settings › Appearance. The stored density SHALL NOT change.
2. While narrow, ⌘B (and the logo button) SHALL open the list for now; widening forgets it.
3. A chosen Minimal stays minimal at every width.
**Quality gates**: `rail-narrow.spec.ts` (fold, restore, compact kept, ⌘B while narrow, stored density untouched, axe on each gallery state); the 390 px captures and the 200 % zoom walk expect the folded rail.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1020/`.
**Visual testing**: Gallery `69-rail-narrow` (folded at 390 and 860, opened at 860; both themes).
**Research basis**: D-25; the 390 px captures (rail cramped); WCAG 1.4.10 reflow.
### AJ-E1021 Triage and Overview fold into Home (D-25 amendment) | Status: DONE-EVIDENCED | Priority P1 | Effort S | Depends: AJ-E1005, AJ-E1014
**Goal**: One place answers "who needs me, what's parked, what's running, what's left to look at" — Home — instead of three screens that repeat each other.
**User story**: As a developer starting the day with a dozen sessions, I want Home to be my triage: the blockers, the parked work with Resume when it's ready, and what ended that I haven't looked at, all from the keyboard.
**Requirements**:
1. Home SHALL show "Parked until a reset" when a session is parked (how long until the reset, or that it's reached; Resume once reached), and a parked session SHALL NOT show under Needs you.
2. "Pick up where you left off" SHALL list what ended in the last 7 days and isn't reviewed or archived: three, then Show all; each has Open and Mark reviewed (recorded in its history, AJ-E1007).
3. Home SHALL be keyboard operable: J/K or arrows move between rows, Enter opens, A runs the row's action; one quiet line names the keys.
4. The ledger-degraded banner SHALL show on Home, only while process tracking is degraded.
5. Triage and Overview SHALL leave More; the decision log records the evidence and the amendment. AJ-E1005's requirements are met on Home; sorting is the rail's.
**Quality gates**: vitest (`home.test.tsx`: grouping, parked, reviewed, keyboard, 50 sessions < 300 ms); the a11y triage-on-Home test (axe 0, J/K); the full Playwright suite; baselines re-rendered with notes, Triage's and Overview's removed.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1021/`.
**Visual testing**: Gallery `55-triage` re-captured on Home (populated and empty, both themes and densities); `17-ledger` on Home.
**Research basis**: D-25 (one place per question; calm until it matters); the overlap measured in the amendment.
### AJ-E1022 Add account without the old wizard: a fresh account, a folder (D-25) | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E1013, AJ-E1016, AJ-E307
**Goal**: Every way into a new account is a short step of first run's own frame; the 2.4k-line account wizard is retired.
**User story**: As a developer adding my third subscription, I want to name it and sign in — or point at a folder and bring it in — without a gate, a proposal or a policy question.
**Requirements**:
1. **A fresh account** SHALL be: a name (a free one filled in, e.g. "Claude Code 2"; focused; Enter creates), Create and sign in, then the same sign-in step first run uses. Back returns to the list when there was one; "Import from a folder instead…" is one link away.
2. **A folder** SHALL be: the system's folder picker (cancel changes nothing), then a check before anything is copied — the path, file count and size, signed in or not, the Keychain note when its sign-in can't move, the breakdown behind "What's in it" — then Bring it in (a copy; the folder is untouched) and its sign-in when needed.
3. The folder check SHALL say plainly when a folder isn't a setup (role=alert, no Bring it in), when it's already in AI Juggler, when its history is too large (only "Bring in settings only" or "Add a fresh account instead"), when it was already brought in ("Already imported as «…»", Done), and when it moved or vanished since the pick (pick it again).
4. First run, Accounts › Add account (straight to a fresh account when nothing new is on this Mac) and New project's no-account detour (over the sheet, which keeps its folder) SHALL all use these steps. Nothing in them asks how packs handle secrets (D-14: the first switch asks).
5. Funnel events SHALL carry no paths, labels or credentials. Flow (d) stays ≤ 3 primary clicks (Add account → Create and sign in → Sign in).
6. The wizard and what only it used (the install gate, candidate cards, set proposal, isolation checklist, the first-run pack question and renewal date, the `__AIJ_FIRST_RUN__` seam, `recommendationFor`/`scanCancel`/`isolationCheck`, the `PackPolicyChooser` component) SHALL be removed with their tests; behaviour that survives keeps a test.
**Quality gates**: vitest (`first-run.test.tsx` fresh and folder steps, `first-run-policy.test.tsx` D-14 and funnel, `new-project-sheet.test.tsx`); `flows-cde.spec.ts` (d); `keyboard-flows.spec.ts` (first run and Add account by keyboard); a11y folder check; captures 39-import, 37-flows/b-detour, 71-add-account; the full Playwright suite.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1022/`.
**Visual testing**: Gallery `39-import` (normal, oversize, not a setup, duplicate, moved, the sign-in after), `37-flows/b-detour`, `71-add-account`.
**Research basis**: D-25 (first run in three steps; depth on request); the inventory of the wizard's tests in the PR.

### AJ-E1023 Choosing a tool inside Add account (D-25) | Status: DONE-EVIDENCED | Priority P1 | Effort S | Depends: AJ-E306, AJ-E1022
**Goal**: "Add Provider" stops being a separate button; Add account asks which tool only when more than one is installed.
**User story**: As someone with only Claude Code, I never see a tool picker; once Codex is installed, Add account asks which tool the account is for.
**Requirements**:
1. Accounts SHALL have one Add account button. With more than one tool installed (or found but unverified), it SHALL first show the tool cards (install state, verified range, kind, a desktop tool's one-session limit, Install → for missing ones); choosing one opens Add account for that tool (its scan, a fresh account named after it, its sign-in).
2. With one tool, Add account SHALL open directly, with no choice shown.
3. The card sheet is a labelled modal dialog that takes focus and closes on Escape; flow (c) stays ≤ 4 primary clicks (Add account → the tool → Create and sign in → Sign in).
**Quality gates**: vitest `add-provider-sheet.test.tsx`; `flows-cde.spec.ts` (c); the a11y tool-choice scan (keyboard Enter on a card); capture 38-flows-cde/c-provider-cards.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1023/`.
**Research basis**: D-25 (progressive disclosure); the Codex adapter in progress (another session) — no adapter code touched.

### AJ-E1024 Use setups where they are; partial bring-in is the user's call (ADR-0018) | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E1018, AJ-E307
**Goal**: Bringing in a setup doesn't make a second, diverging world, and one failure never traps the user.
**User story**: As someone with six Claude setups, I want AI Juggler to work with the ones I have (same sign-in, same history as my Claude app) unless I ask for a copy, and when one of six can't come in, to go on with the five.
**Requirements**:
1. The bring-in screen SHALL offer, in view, "Use them where they are" (default, recommended) and "Make separate copies", one line each on what it means.
2. In place, AI Juggler SHALL write nothing of its own into the folder: no marker, no permission change, no `settings.json` edits, no status line wrapper or spool. Its launch settings and capture SHALL live under the data dir's `overlays/<account-id>/` and go in with `claude --settings`.
3. A profile's model SHALL go in as `claude --model` for every account; `settings.json` SHALL never be rewritten for it (it wiped hooks and permissions in copies).
4. The default `~/.claude` in place SHALL launch and sign in with `CLAUDE_CONFIG_DIR` unset (its Keychain entry and `~/.claude.json`); any other folder with its exact path.
5. Each pick SHALL come in or not on its own. The result SHALL list the failed ones in plain words with Try again (those only), "Use it where it is instead" (a failed copy) and Continue with the ones that came in; sign-ins of the ones that came in survive retries.
6. While bringing in, Close and Esc SHALL mean Stop: the setup in progress finishes, what came in stays, and the result offers Bring in the rest or Continue.
7. A copy whose account record can't be saved SHALL remove its copied folder. In-place folders SHALL count as "Already in AI Juggler" in the scan; the isolation checklist SHALL describe them honestly; the Accounts card SHALL say "uses ~/x where it is" or "a copy of ~/x".
8. A copied account SHALL be able to switch to its original folder ("Use the original instead", confirmed, nothing deleted), refused while a session runs on it or when another account already uses that folder.
9. Launches and sign-ins SHALL NOT inherit `ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN` or `CLAUDE_CODE_OAUTH_TOKEN` from AI Juggler's own environment.
**Quality gates**: Rust `in_place_accounts` (nothing in the folder changes — sizes and mtimes — through bring-in, launch, the usage pass and the checklist; `--model`/`--settings`; a copy's settings survive a model launch; our own copy and non-setups refused), `vendor_default_folder`, the supervisor's env-unset test; vitest `first-run.test.tsx` (default in place, Continue, Try again, use in place instead, Stop) and the placement words; the full Playwright suite; the accessibility gate; walked by hand.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1024/`.
**Research basis**: maintainer, 2026-10-20: "most of people don't want 10s of different profiles confusing everywhere"; "we should always give option to the user if they want to continue with partial import or try again. We can't be forceful to users". ADR-0018.

### AJ-E1025 Plain words in the inspector views; no task codes on screen (D-25) | Status: DONE-EVIDENCED | Priority P1 | Effort S | Depends: AJ-E407, AJ-E408, AJ-E1019
**Goal**: The session, account, project and tool views say what they show in plain words. No task or decision code, raw id or "No data" placeholder reaches the screen, and the tool view gets a way in.
**User story**: As someone opening a session's Usage tab, I read "No usage measured for this yet", not "lands with AJ-E606+ (two-clock usage engine)".
**Requirements**:
1. No task code (`AJ-E…`, `E3+`, `(E2)`) or decision code (`D-08`) SHALL appear in visible text, a visible attribute or a user-facing string; `scripts/terminology_lint.sh` SHALL fail on one (JSX text, wrapped JSX text, attributes, strings) and leave comments alone. Its selftest covers each form.
2. Each inspector tab's honest empty SHALL say what's missing in plain words. An action that isn't built yet SHALL NOT be shown; one that exists but doesn't apply now stays disabled with its reason (AJ-E407 Req.4 as amended). A row with nothing to show waits until it has data.
3. The views SHALL name a tool by its name ("Claude Code", never `claude-code`), a state in the rail's words ("Hit its limit", never `quota_blocked`), and a session on screen as a session.
4. The tool view SHALL read what's installed ("Version 2.1.296 · tested with 2.1.x", "Not installed on this Mac"), and SHALL open from the tool chip in a session's or an account's header ("About Claude Code"). The header SHALL name the tool or account it shows.
**Quality gates**: the terminology lint (and its selftest); vitest `inspector-*.test.tsx`, `tier-toggles`, `quality-tab`, `running-model`, `renewal`; capture `72-tool-view` (axe on every capture); the visual gate.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1025/`.
**Research basis**: D-25 (plain words, progressive disclosure); D-06 (the six tabs and honest empties stay); the maintainer's lesson: walk every changed control in the browser before calling it done.
