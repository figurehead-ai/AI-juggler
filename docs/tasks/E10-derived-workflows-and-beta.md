# E10 — Derived Workflows, Safety Nets and Beta Operations: Implementation Tasks

> Epic owner: Product + Core. Status: TODO (all tasks).
> Settled law — do not re-litigate D-01..D-15 in
> `docs/review/decision-log.md`. Sources: pass2/07 W1-W11 derived
> workflows, pass2/06 external pain research, deep/07 personas,
> deep/10 proposal. Items marked `[inferred]` in their sources
> SHALL NOT be built until their falsification task here records
> dogfood or interview evidence (AJ-E809 consumes the results).
> Vocabulary: Provider / Account / Instance / Launch settings.

### AJ-E1001 Per-Instance identity assertion (wrong-account billing guard) | Status: TODO | Priority P0 | Effort M | Depends: —
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

### AJ-E1002 Git worktree isolation for parallel Instances | Status: TODO | Priority P0 | Effort L | Depends: AJ-E1001
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

### AJ-E1003 Quota reset queue and parked continuation | Status: TODO | Priority P1 | Effort M | Depends: AJ-E1001
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

### AJ-E1004 Overnight and unattended run supervision | Status: TODO | Priority P1 | Effort M | Depends: AJ-E1003
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

### AJ-E1005 Morning triage and multi-project day board | Status: TODO | Priority P1 | Effort M | Depends: AJ-E1004
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

### AJ-E1006 Pair review and model arbitrage workflows [inferred — falsify first] | Status: TODO | Priority P2 | Effort M | Depends: AJ-E1005
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

### AJ-E1007 Session lifecycle: search, archive, replay, retention | Status: TODO | Priority P1 | Effort M | Depends: AJ-E1005
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

### AJ-E1008 Provider outage failover day [inferred — falsify first] | Status: TODO | Priority P2 | Effort M | Depends: AJ-E1006
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

### AJ-E1009 Backup, restore and machine migration of Juggler's own data | Status: TODO | Priority P1 | Effort M | Depends: AJ-E1007
**Goal**: Let users export and restore Juggler's database, packs and settings so a new machine or a corrupt store never costs them their history.
**User story**: As a user moving laptops, I want one export file I can carry over and restore, with proof the restore matches.
**Requirements**:
1. Export SHALL produce a single archive (DB snapshot, packs, settings, lineage) with a manifest and checksums; credentials and provider login material SHALL be excluded by construction and the exclusion verified by the AJ-E801 redaction check.
2. Restore SHALL verify checksums and schema version, refuse incompatible versions with a named reason, and restore into a clean managed directory without touching pre-existing vendor installs (invariant).
3. A restore rehearsal SHALL run in CI on a fixture archive: restore, launch read-model, assert counts and lineage integrity match the manifest.
4. Fleet machines SHALL each keep independent backups; restoring one machine's backup onto another requires an explicit machine-identity warning (E7 identity).
**UI/UX reasoning**: NN-g #5/#9: destructive-adjacent flows state scope, verify first, and report in plain language; progress and completion are visible, never silent (visibility of status).
**Performance budget**: Export <60 s for 1 GB store (streaming, progress shown); restore verification <30 s; no background upload anywhere (local file only).
**Quality gates**: Fixture restore test green in `just verify`; credential-pattern scan of a real export structure passes (patterns only, no real secrets).
**Evidence/validation proof**: Store rehearsal log + manifest diff in `docs/qa/evidence/AJ-E1009/`; pass = counts/checksums match, dogfood migration of one real (consented, redacted) store recorded.
**Visual testing**: Capture export progress, restore verify result and machine-identity warning at 1280x800 both themes; pass = outcome text explicit, failure states name the file and reason.
**Files to touch**: `crates/juggler-core/` (export/import), `crates/juggler-app/` (commands), settings UI, E7 identity model.
**Research basis**: first-report F1.7; deep/10 backup proposal; SPEC §12 storage.
**Handover note**: Export format versioning is an ADR-worthy decision if the schema is non-trivial; raise it rather than inventing a silent format.

### AJ-E1010 Updates and release integrity for beta | Status: TODO | Priority P1 | Effort M | Depends: AJ-E1009
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