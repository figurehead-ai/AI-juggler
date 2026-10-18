# E4 — Indicators and Right Inspector: Implementation Tasks

> Epic owner: daily supervision surface. Settled law: D-06 (full inspector,
> honest fields), D-07 (measurable signals only, no composite score), D-12
> (user-entered optional subscription renewal date). Source spec:
> `docs/review/pass2/04-daily-indicators-and-right-inspector.md` (§§3-6).
> Do NOT re-litigate D-01..D-15 in `docs/review/decision-log.md`.
> Vocabulary (D-02/D-10): Provider=HarnessId, Account, Instance=
> SessionRecord+kind, Session=conversation inside an Instance,
> Profile demoted to "Launch settings" (never a rail/inspector noun).
> Honest-fields rule (D-06): a field appears only when actually measurable;
> otherwise it renders `unavailable` + reason, never a dressed-up estimate.
> Fleet-shaped from day one (D-13): every filter/sort key includes `machine`.
> All tasks: Status TODO until the named evidence artifact exists and passes.

### AJ-E401 Enrich the Instance read model (I-01..I-11 data contract) | Status: DONE-EVIDENCED (zcode-primary, 2026-10-13) | Priority P0 | Effort L | Depends: none
**Goal**: Extend `SessionInfo` so one payload carries every indicator-row and inspector field, with explicit absence instead of fabricated values.
**User story**: As a multi-account developer (Persona A, PROPOSAL.md:51), I want the rail to know my account, model, quota, and resources without clicking each Instance, so that I can spot the quota wall at a glance.
**Requirements**:
1. The backend SHALL add to `SessionInfo` (ts-rs generated, never hand-edited in `ui/src/types/generated/`): `account_label`, `account_id`, `model` (nullable), `quota` (nullable `QuotaWindow` + `Confidence`), `tokens`, `cpu_pct`/`rss_bytes` (nullable), `child_count`, `subtask_count`, `git_branch`/`git_dirty_count` (nullable), `needs_attention` (nullable reason enum), `last_activity_ms`, `machine_id`.
2. Absent data SHALL serialise as `null`/absent, never `0`, `""`, or a guessed model name; unknown harness SHALL map to a generic marker upstream, never blank.
3. `session_infos()` SHALL join Profile->Account and `SettingsOverlay.model` without N+1 storage reads per row (batch or denormalise at write time).
4. The bridge SHALL keep `sessions_updated` event-driven (<300 ms) and add a 30 s quota refresh tick and 5 s resource tick as separate, coalesced emissions.
5. ts-rs generation SHALL be regenerated and committed; a Rust test SHALL assert the generated TS contains every new field name.
**UI/UX reasoning**: Nielsen/NN-g visibility of system status (heuristic 1): status must be perceivable without interaction. No visual change here, but nullability is the honesty contract the UI renders (pass2/04 §3 stale rule, §6).
**Performance budget**: `sessions_list` p95 < 20 ms at 50 Instances; payload < 8 KB/row; resource tick CPU < 1% (SPEC.md:24); no per-row subprocess spawns on the hot path.
**Quality gates**: `just verify` green; clippy `-D warnings`; no new `unwrap()` in commands path; QUALITY.md DoD.
**Evidence/validation proof**: `cargo test -p juggler-app session_info` with fixtures (full row, quota-less row, model-less row, unknown status) asserting null-not-zero; output + `git diff ui/src/types/generated/SessionInfo.ts` stored in PR description; pass = all fixtures green and diff shows only additive fields.
**Visual testing**: None (data contract); screenshot proof deferred to AJ-E402. State in PR: "no visual surface changed".
**Files to touch**: `crates/juggler-app/src/commands.rs:108,911`, `crates/juggler-core/src/model.rs:263,279,288,389`, `ui/src/types/generated/SessionInfo.ts` (generated), `ui/src/lib/bridge.ts:303,325`, `ui/src/lib/bridge.ts` MOCK_SESSIONS (:65).
*Implementation mapping (corrected in this task's PR per the claiming protocol — the task text was written against the pre-E104/E107 tree)*:
- `SessionInfo` is now `InstanceInfo` (AJ-E107); its `label` field WAS the account label, so Req.1's `account_label` lands as a rename of `label` (serde alias `label` kept one version), and the mechanical rename sites are `ui/src/App.tsx`, `ui/src/components/WorkflowRail.tsx`, `ui/src/screens/WorkPanels.tsx`, `ui/tests/visual/fixtures.ts` — no visual surface changed.
- `crates/juggler-core/src/model.rs` needed NO change: post-E104 the record already carries `machine`, `last_activity`, `usage`; quota/confidence/token wire mirrors (`QuotaSnapshotInfo`, `TokenCountsInfo`) live in `commands.rs` per the established `*Info` flat-wire pattern.
- Added `crates/juggler-supervisor/src/supervisor.rs` (`live_stats` batched API + `LiveStatsCollector`: `descendants()` refreshes the whole process table per call — per-row use would blow the 20 ms budget), `crates/juggler-app/src/state.rs` (collector in `AppState`), `crates/juggler-app/src/lib.rs` (5 s resource + 30 s quota tick threads beside the lifecycle forwarder).
- Drive-by wire fix, same contract: `ProjectInfo.last_activity_ms` and the new nullable numerics emit `number | null` (the old `#[ts(type = "number")]` on `Option` fields erased nullability — a lying type).
**Research basis**: pass2/04 §3 table I-01..I-11 (lines 175-185); `SessionInfo.ts:3-38` (8 fields today); `model.rs:145,164,263,279,288`.
**Handover note**: Trap: `ProfileInfo.account_label`/`model` exist (ProfileInfo.ts:15,23) but are NOT on SessionInfo today — do not invent a second join in the UI. If a field cannot be sourced yet, ship it nullable; never backfill with estimates.

### AJ-E402 Render the 11-field indicator row and card variant | Status: DONE-EVIDENCED (zcode-primary, 2026-10-13) | Priority P0 | Effort L | Depends: AJ-E401
**Goal**: Rewrite `SessionRow` to render I-01..I-11 in the pass2/04 §3 order, plus a two-line card variant with identical fields.
**User story**: As Persona A with 8 running Instances, I want provider, account, model, quota, cost, resources, children, branch, and attention state on every row, so that I never open the wrong Instance.
**Requirements**:
1. Every row SHALL render, in order: I-01 Provider icon+abbrev, I-02 Account initials disc+hue+label, I-03 model chip, I-04 state icon+text chip, I-05 quota bar+% left+reset countdown+tier tag, I-06 tokens/cost, I-07 CPU/RAM sparkline+text, I-08 child/subtask badges, I-09 branch+dirty, I-10 needs-attention badge, I-11 age/last-activity.
2. Missing data SHALL render exactly: I-05 dashed bar + `Quota: no data`; I-06 tokens-only when pricing unknown (never `$0.00`); I-03 `default model`; I-09 omitted entirely for non-git cwd; I-08 hidden at 0.
3. The fabricated `percent={0}` rings (App.tsx:513-514) SHALL be replaced by the honest I-05 rendering wherever this row/card is reused.
4. The card variant SHALL show the same 11 fields (5-min sparkline) — no number may exist on a card that the rail cannot explain.
5. Every state SHALL be icon+text+colour (never colour-only); unknown wire status SHALL render `Unknown`, never a neutral dot.
**UI/UX reasoning**: WCAG 2.2 1.4.1/1.4.3 (no colour-only meaning, contrast >=4.5:1) and NN-g recognition-over-recall: quota tier tags `Meas/Comp/Est` stay adjacent to the number (pass2/04 §4.1). Copy exact: `Quota: no data`, `default model`, `Needs you`, `Stuck?`, `Quota blocked`.
**Performance budget**: Row render < 4 ms; list of 50 rows paints < 100 ms; sparkline repaint at most 5 s; virtualise past 50 rows (no silent `slice(0,N)` caps).
**Quality gates**: `just verify`; component tests for all 9 states + 5 empty states; axe-core 0 critical/serious on the rail fixture page.
**Evidence/validation proof**: Vitest/Jest (repo harness) `SessionRow.test` with 14 fixtures (9 states, no-quota, no-pricing, non-git, 0 children, unknown status) asserting exact strings above; stored in CI log; pass = 14/14 and no `$0.00`/`percent 0` string in snapshot.
**Visual testing**: Screenshots at 1440x900 and 390x844 (responsive rail collapse), Comfortable density, fixtures: running+quota, needs-you, quota-blocked, no-data row; compare by pixel-diff (Playwright) tolerance 1%; screen-reader pass: every number reachable as text, `aria-label` contains full field text. Pass = diff <1% vs approved baseline + axe clean.
**Files to touch**: `ui/src/components/WorkflowRail.tsx:295,397,429`, `ui/src/components/primitives.tsx:165`, `ui/src/components/icons.tsx:22`, `ui/src/App.tsx:442,513`, `ui/src/lib/bridge.ts:65`.
*Implementation mapping (corrected in this task's PR per the claiming protocol — the task assumed a Vitest/Jest harness that did not exist yet)*:
- Added `ui/src/components/indicator.tsx` — the single I-01..I-11 display-mapping module shared by row, card and tests (row/card parity is structural, not convention).
- Component-test harness added: `ui/vitest.config.ts`, `ui/src/components/SessionRow.test.tsx` (25 tests over the 14 named fixtures), devDeps in `ui/package.json` (vitest, @testing-library/*, jsdom); `pnpm test` wired into the CI ui job (gate STRENGTHENED, recorded in the evidence manifest).
- `ui/tests/visual/a11y.spec.ts` — axe-core 0 critical/serious gate in both themes (D-16) + a no-overflow layout gate (rows must never spill their rail width; provider abbrev/model/age/resource text yield first, chips and discs never clip).
- `ui/src/styles.css` gains the `.acct-disc` per-Account hue utility (hue is data from a stable id hash; both themes fixed via tokens per D-16).
- The rail's legacy `slice(0,6/8/15)` caps are deliberately LEFT for AJ-E404 Req.7 (deleted together with virtualisation); noted here so it is not read as a miss.
**Research basis**: pass2/04 §3 (lines 163-192), §7.1 wireframe (438-460); deep 01 B-11/B-14; `WorkflowRail.tsx:429` (label+dot today).
**Handover note**: Provider glyphs (Claude/Codex/ZCode) do not exist in icons.tsx:22 (8 generic glyphs) — add marks, do not reuse status dots. Trap: do not sort or filter here; AJ-E404 owns ranking.

### AJ-E403 Density modes, legend, and the stale-data rule | Status: DONE-EVIDENCED (zcode-primary, 2026-10-13) | Priority P1 | Effort M | Depends: AJ-E402
**Goal**: Ship Comfortable/Compact/Minimal densities, the shared state/quota legend, and dimmed `stale - Nm` rendering for expired values.
**User story**: As a developer monitoring while coding, I want a collapsed icon rail that still screams "needs you", so that supervision costs me no screen space.
**Requirements**:
1. Density SHALL persist per user in `localStorage` key `aij.density` with values `comfortable|compact|minimal`, default `comfortable`.
2. Comfortable (~56 px, 3 lines) SHALL show I-01..I-11; Compact (~36 px, 2 lines) SHALL show I-01, I-02, I-04, I-05 bar-only, I-09, I-10 plus CPU/RAM as text; Minimal (60 px icon rail) SHALL show avatar disc + state glyph + `!` count only.
3. Density SHALL never hide I-10 or the I-04 text in Comfortable/Compact; Minimal SHALL keep glyph+badge and expose full I-01..I-11 text via tooltip and `aria-label`.
4. Switching SHALL be a header segmented control plus `Cmd/Ctrl+D` cycling; the inspector SHALL NOT change with density.
5. Any value older than 2x its refresh interval SHALL render dimmed with caption `stale - Nm` (minutes, integer); it SHALL never render as fresh.
6. The legend (9 states, quota hatch=tier, sparkline solid=RAM/outline=CPU) SHALL appear identically in rail-footer `?` popover, Inspector Overview header, and the empty-Inbox "All clear" explainer.
**UI/UX reasoning**: Apple HIG (consistency and standards; persistent user preference) and WCAG 2.2 1.3.3 (instructions not by shape/colour alone). Shortcut `Cmd/Ctrl+D` must not collide with browser bookmark on web preview — use Tauri menu accelerator in app, `Ctrl+D` only. Copy exact: legend strings from pass2/04 §4.1 table.
**Performance budget**: Density switch re-layout < 50 ms at 50 rows; stale check runs on the existing 15 s tick, no new timers (< 1 extra wakeup/min); legend popover opens < 100 ms.
**Quality gates**: `just verify`; unit tests for stale threshold (119 s fresh at 60 s interval, 121 s stale); keyboard-only density cycle test.
**Evidence/validation proof**: Test command `cargo test --workspace` (backend unchanged) + UI test `density.test` asserting field presence per mode from a 3-row fixture; pass = all assertions green; evidence (test log) linked in PR; manual dogfood note in `docs/qa/` not required.
**Visual testing**: Screenshots: 3 densities x (1440x900, 1024x768), plus Minimal tooltip open, plus one stale row; method: Playwright pixel-diff vs baseline tolerance 1% + manual contrast check on dimmed stale text (>= 4.5:1 against background, else keep text colour and add `stale` chip instead of dimming). Pass = baselines approved by reviewer, axe 0 critical.
**Files to touch**: `ui/src/components/WorkflowRail.tsx:110,397`, `ui/src/App.tsx:62`, `ui/src/components/primitives.tsx`, `ui/src/styles.css`.
*Implementation mapping (corrected in this task's PR per the claiming protocol)*:
- Added `ui/src/lib/density.ts` (persistence + migration + keyboard predicate + stale threshold — pure, unit-tested) and `ui/src/components/density.test.tsx`.
- `ui/src/components/indicator.tsx` gains the shared `IndicatorLegend`, the contrast-safe `StaleChip`, dual resource sampling (RAM solid + CPU outline — the legend never describes what is not drawn) and `isActiveStatus`.
- Legend surfaces TODAY: rail-footer `?` popover, Overview screen header (the Inspector does not exist until AJ-E406 — the component is shared and drops in then), and the rail's All-clear empty state.
- The "existing 15 s tick" did not exist; ONE shared 15 s interval in App drives all staleness recomputation (no per-value timers). Terminal rows hide resource slots entirely (a finished Instance is not "unavailable" — active-unsampled keeps the honest `—`); the minimal rail keeps ALL active statuses visible (waiting/stuck/quota-blocked included — the collapsed rail must still scream "needs you").
- `⌘B` keeps its legacy minimal↔comfortable toggle; the cycle shortcut is Ctrl+D everywhere, Cmd+D only inside the Tauri app (web preview keeps the browser bookmark shortcut).
**Research basis**: pass2/04 §4.1-4.2 (lines 198-241); `WorkflowRail.tsx:110` (60 px collapsed rail exists); SPEC.md:20 (20-session target drives Compact).
**Handover note**: Trap: existing `aij.rail.collapsed` (App.tsx:62) is a boolean — migrate it into `aij.density=minimal` on first read, do not keep two competing keys.

### AJ-E404 Needs-me-first sorting, filtering, search, and snooze | Status: DONE-EVIDENCED (zcode-primary, 2026-10-13) | Priority P0 | Effort M | Depends: AJ-E402,AJ-E403
**Goal**: Replace silent list caps with Needs-me-first sort, multi-filter chips, `Cmd/Ctrl+K` search, group-by, and I-10 snooze/mute.
**User story**: As Persona A returning to the desk, I want the Instance that has waited longest for me sorted first in every view, so that I answer the blocked agent before checking anything else.
**Requirements**:
1. Default sort SHALL be `Needs me`: Failed > QuotaBlocked > WaitingUser (waiting duration descending) > Stuck > Running > rest; sort menu SHALL also offer Quota risk, Project, Provider, Account, Age, Name.
2. Quota-risk sort SHALL order by lowest `% left` x burn rate; unknown-quota Instances SHALL group last under header `no data`.
3. Filter chips SHALL be `All`, `Needs me (n)`, `Running`, `Ended` (multi-toggle with counts) ANDed with single-select Provider/Account/Project dropdowns; every key SHALL include `machine` (D-13) even with one local machine.
4. Search (`Cmd/Ctrl+K` focus, `Esc` clear) SHALL match label, project, branch (I-09), model (I-03), and account; empty result SHALL render `No instances match` + `Clear filters` button.
5. Group-by (None/Project/Account/Provider) SHALL render group headers with rollup `N running - M needs you - lowest quota X%`; Needs-me sort SHALL apply inside groups.
6. Snooze SHALL offer `Snooze 15m / 1h / Until reset` and `Mute this instance` on the I-10 context menu; snoozed items SHALL sink below unsnoozed peers, never disappear; rail badge SHALL count unsnoozed only.
7. Lists SHALL virtualise past 50 rows; the legacy `slice(0,6/8/15)` caps SHALL be deleted.
**UI/UX reasoning**: NN-g visibility + user control (snooze is reversible, mute is per-instance, both labelled with remaining time `snoozed 12m`). Nielsen heuristic 6: counts on chips prevent filter-state amnesia. Shortcuts: `Cmd/Ctrl+K`, `Esc`; chips are real `<button aria-pressed>`.
**Performance budget**: Sort+filter of 200 rows < 16 ms (one frame); search keystroke-to-paint < 50 ms; virtualised scroll 60 fps at 200 rows; snooze persistence write < 5 ms.
**Quality gates**: `just verify`; sort-order property test (random 200-row fixture, severity invariant holds for every adjacent pair); no regression to AJ-E402 snapshots.
**Evidence/validation proof**: UI test `sorting.test`: fixed 8-row fixture covering all severities + 1 snoozed + 1 no-quota row, asserting exact rendered order (IDs listed in test); pass = exact order match; plus manual live-dogfood: launch fake harness in WaitingUser, confirm it reaches row 1 within 300 ms of the event; record fixture IDs in PR.
**Visual testing**: Screenshots 1440x900: default Needs-me list, Quota-risk list with `no data` group header, group-by Project with rollups, empty-search state, snoozed badge (`snoozed 12m`); Playwright diff tolerance 1%; a11y: chip counts announced (`Needs me, 3 items`), focus returns to search on `Esc`. Pass = reviewer-approved baselines + axe 0 critical.
**Files to touch**: `ui/src/components/WorkflowRail.tsx:243-395`, `ui/src/lib/bridge.ts:303`, `ui/src/App.tsx`, `ui/src/styles.css`.
*Implementation mapping (corrected in this task's PR per the claiming protocol)*:
- Added `ui/src/lib/railview.ts` (pure sort/filter/search/group/snooze logic + `aij.snooze` persistence) and `ui/src/components/sorting.test.tsx`.
- The `machine` filter dimension exists in the model from day one (D-13); no visible machine dropdown renders while only `this-mac` exists (a one-machine dropdown is noise — recorded here).
- The live-dogfood step ("fake harness reaches row 1 within 300 ms of a WaitingUser event") is simulated through the mocked bridge: WaitingUser EMISSION does not exist until AJ-E405 — the event→row-1 path is asserted on a dispatched `session://updated` payload instead; the live-binary run moves to AJ-E405's evidence.
- Virtualisation windows the flat list past 50 rows; grouped triage views render fully (the 200-row scale is ungrouped monitoring) — every row stays reachable by scroll in both modes.
- Drive-by a11y fix: the needs-you tab badge is contrast-safe in both themes (`.attn-badge` utility; black-on-amber failed 4.5:1 in light) and never widens its tab (the overflow gate caught both).
**Research basis**: pass2/04 §4.3 (lines 243-264); deep 02 WR-02/WR-05 (silent caps); full review UX.3 (snooze/mute); `model.rs:398` (`machine` field).
**Handover note**: Trap: Inbox becomes a filter preset (`Needs me` chip), not a destination tab — do not build a separate Inbox page. Burn rate for req 2 comes from AJ-E409 signals; until then use quota % left alone and label the sort `Quota risk (quota only)` in the menu.

### AJ-E405 Emit Stuck, WaitingUser, and QuotaBlocked (needs-attention upstream) | Status: DONE-EVIDENCED (zcode-primary, 2026-10-13) | Priority P0 | Effort L | Depends: AJ-E401
**Goal**: Wire the supervisor/adapter heuristics so `SessionStatus::Stuck|WaitingUser|QuotaBlocked` are actually emitted with reason + waiting duration — the states I-04/I-10 render.
**User story**: As Persona A, I want a hung agent flagged `Stuck?` with the evidence (`no output 6m - CPU 0.3%`), so that I trust the badge and act instead of polling terminals.
**Requirements**:
1. The heuristic engine SHALL evaluate, per SPEC §5.1: (a) no output AND cpu < threshold AND transcript mtime stale for N min -> `Stuck`; (b) permission/wait prompt alive > N min -> `WaitingUser`; (c) non-zero exit with rate-limit signature -> `QuotaBlocked`; thresholds SHALL be user-tunable in Settings.
2. Each emission SHALL carry a structured reason (rule id, measured inputs, since timestamp) stored on the record and sent over `sessions_updated`; the UI SHALL never re-derive the state from raw status alone.
3. Waiting duration SHALL be computed from `last_activity`/state-since and tick in the UI every 15 s without new backend polls.
4. Transitions SHALL be debounced: a state must hold for 2 consecutive evaluations (>= 30 s apart) before emitting, except `QuotaBlocked` on exit which emits immediately.
5. Recovery SHALL emit the return to `Running`/`Finished` within 300 ms of output resuming or exit classification.
6. The emission SHALL be fleet-shaped: the event payload includes `machine` (D-13) so a remote board ranks identically.
7. No automatic action SHALL follow any emission (no auto-kill/restart/handover) — SPEC §7 passive-automation invariant; suggestions only.
**UI/UX reasoning**: Nielsen/NN-g trust calibration: `Stuck?` keeps its `?` permanently (heuristic, pass2/04 §6) and the badge tooltip lists the tripped rule + inputs. WCAG 4.1.3: state changes are announced via polite live region, not colour flash alone. Copy exact: `Stuck?`, `Needs you`, `Quota blocked`, snooze strings from AJ-E404.
**Performance budget**: Heuristic evaluation per Instance < 2 ms on a 15 s cycle; fleet of 50 Instances < 100 ms/cycle, < 1% CPU total; event latency state-change-to-UI < 300 ms (EPICS.md:66).
**Quality gates**: `just verify`; fixture corpus (fake harness: hang / wait-permission / quota-exit / crash) classified >= 90% precision and recall (EPICS M2-5); no guardrail test regressions (`tests/guardrails.rs`).
**Evidence/validation proof**: Command `cargo test --workspace stuck waiting quota` (name filters per repo convention) on committed fake-harness scripts; pass = precision/recall >= 90% computed in the test report and stored under `docs/qa/e4-attention-fixtures.md`; live dogfood: run the hang fixture, screenshot I-10 appearing <= 15 s after threshold; both artifacts linked in PR.
**Visual testing**: Screenshots 1440x900: row in each of WaitingUser (with `wait 4m`), Stuck? (dashed border), QuotaBlocked (reset countdown); tooltip open showing rule inputs; comparison Playwright diff tolerance 1% vs AJ-E402 baseline rows; a11y: live-region announcement text asserted in DOM, badge text not icon-only. Pass = reviewer confirms reason text matches the fixture's known cause.
**Files to touch**: `crates/juggler-supervisor/src/supervisor.rs:474,557`, `crates/juggler-supervisor/src/lifecycle.rs`, `crates/juggler-core/src/model.rs:188-258,407-411`, `crates/juggler-app/src/commands.rs:782,817`, `crates/juggler-adapters/src/fake.rs`, `ui/src/lib/bridge.ts:325`.
*Implementation mapping (corrected in this task's PR per the claiming protocol)*:
- The engine lives in `crates/juggler-supervisor/src/heuristics.rs` (new): pure over injected inputs/clocks; `lifecycle.rs` needed NO change (the app applies transitions directly — the forwarder classifies before mapping). `fake.rs` untouched; the fixture modes landed in the `fake-harness` BINARY (`--wait-prompt`, `--quota-exit`).
- Storage: schema v6 adds `instances.attention_json` (additive, nullable); `InstanceRecord.attention: Option<AttentionReason>`. Wire: `InstanceInfo.attention` (rule + measured inputs + since) — the UI badge tooltip and `aria-label` render it verbatim and never re-derive it.
- Corpus + live dogfood: `crates/juggler-app/tests/attention_corpus.rs` (100% precision/recall; `docs/qa/e4-attention-fixtures.md`). Debounce note: Req.4's 30 s law supersedes "≤15 s after threshold" — the emission lands at the confirmation evaluation (app worst case + one 15 s tick); recorded in the evidence doc.
- Thresholds persist in `heuristics.json` (app data dir) with SPEC §5.1 defaults; the Settings SCREEN UI lands with its own task and writes the same file (recorded here so it is not read as a miss).
**Research basis**: pass2/04 §3 I-10 + §6 Stuck row (lines 184, 418); SPEC.md:220-221, §5.1 heuristics (docs/SPEC.md sed 220-234); EPICS.md:79 (M2-5 >= 90%); full review F2.5 ("modelled but never emitted", model.rs:188-207).
**Handover note**: This is the upstream gap every E4 rendering task assumes. If heuristics cannot reach 90% on a fixture class, ship the emission with that class labelled `Stuck?` only and record the shortfall — never silently widen thresholds to pass.

### AJ-E406 Right Inspector shell: 6 tabs x 5 selection types | Status: DONE-EVIDENCED (zcode-primary, 2026-10-13) | Priority P0 | Effort L | Depends: AJ-E401
**Goal**: Build the inspector container that shows Overview/Usage/History/Quality-Performance/Processes/Actions for Session, Provider, Account, Instance, and Project selections (D-06), with the fleet-at-a-glance empty state.
**User story**: As Persona A, I want one stable right-hand home whose tabs never move whatever I click, so that muscle memory replaces hunting through screens.
**Requirements**:
1. Clicking any rail row (Instance/Project) or global card (Provider/Account) or History/search result (Session) SHALL replace the inspector atomically; header SHALL always show selection-type icon, name, I-04 state chip where applicable, and the primary action.
2. All six tabs SHALL exist for all five selection types from day one (D-06); a tab with no measurable data SHALL render its honest empty state (AJ-E409), never be hidden or disabled without adjacent reason text.
3. Tab state SHALL persist per selection type (`aij.inspector.tab.<type>`); switching selection SHALL restore that type's last tab.
4. Empty selection SHALL render the fleet-at-a-glance summary (counts by state, lowest-quota Account, machine gauges per SPEC.md:345), never a blank panel.
5. Every tab footer SHALL show `Updated Ns ago - source: <supervisor|transcript|usage-engine|git>` using the field's real source; stale rule from AJ-E403 SHALL apply per field.
6. Keyboard: `Cmd/Ctrl+1..6` SHALL switch tabs inside the inspector; focus SHALL be trapped logically (header -> tabs -> content) with visible focus rings.
7. The inspector SHALL be fleet-shaped: remote-machine selections render the same tabs with a `machine` chip and read-only Actions until Fleet control lands (D-13).
**UI/UX reasoning**: Apple HIG (stable navigation, predictable layout) + Nielsen heuristic 4 (consistency): tab order is fixed Overview, Usage, History, Quality/Performance, Processes, Actions — note D-06 names the 4th tab "Quality/Performance" while pass2/04 §5 shortens it to "Performance"; use `Quality / Performance` as the label. WCAG 2.4.6 headings/labels: each tab panel has an `aria-labelledby` tab.
**Performance budget**: Selection-to-inspector paint < 100 ms at 50 Instances; tab switch < 50 ms (content pre-fetched per selection); inspector re-render on 5 s resource tick SHALL NOT exceed 8 ms or steal terminal focus.
**Quality gates**: `just verify`; routing unit tests (5 selection types x initial tab); focus-order test with keyboard-only script; no tab unmounts the embedded terminal (TerminalView regression check).
**Evidence/validation proof**: UI test `inspector-shell.test`: programmatically select each of the 5 types from fixtures, assert 6 tabs present, header fields, and per-type tab persistence across a reselection; pass = 30/30 assertions; plus live dogfood step: click Instance -> Project -> Account in the dev app (`just dev`) and record that tab memory holds; log linked in PR.
**Visual testing**: Screenshots at 1440x900 and 1280x800: empty-selection summary, Instance/Overview, Account/Usage, Project/Processes headers; states: populated, honest-empty tab, stale footer; Playwright diff tolerance 1%; a11y axe 0 critical + manual tab-order walkthrough recorded as checklist in PR. Pass = reviewer approves header/tab consistency across all five types.
**Files to touch**: `ui/src/App.tsx`, `ui/src/screens/WorkPanels.tsx`, `ui/src/components/WorkflowRail.tsx:295`, `ui/src/lib/bridge.ts:38,303`, `ui/src/styles.css`, `docs/SPEC.md:343-353` (read only, cite).
*Implementation mapping (corrected in this task's PR per the claiming protocol)*:
- The shell is `ui/src/components/RightInspector.tsx` (new; one name everywhere per the handover note). WorkPanels' ProjectPanel/WorkEmpty bodies were EXTRACTED into it (WorkPanels keeps the login-terminal panel + NewSessionModal only); `ui/src/screens/Profiles.tsx` gained the account-inspect entry (HarnessGlyph button) and Overview's quota cards select their Provider.
- The terminal NEVER unmounts on tab switches: all six panels mount once per selection and toggle `hidden` (also the pre-fetched-content perf model).
- Login terminals keep their dedicated panel (not one of the five types); the Session (conversation) type renders honest empties until AJ-E407 fills it.
- Corpus flake note (AJ-E806): the E405 attention-corpus test flaked twice under cold parallel workspace load (5 s spawn deadline); deadlines widened to 20 s/90 s — three consecutive fully-loaded runs green; recorded in the E406 manifest.
**Research basis**: decision-log D-06 (lines 60-68); pass2/04 §5 intro (lines 268-286) and §5.1-5.5; 00-executive-delta §1.2 item 7 (no Provider panel exists, SPEC.md:343).
**Handover note**: Name the component `RightInspector`, not `SessionPanel`/`SnapshotPanel` (prior-report names) — one name everywhere. Trap: WorkPanels.tsx today hosts panel content; extract, do not duplicate, the shell.

### AJ-E407 Inspector content for Instance and Session selections | Status: DONE-EVIDENCED (zcode-primary, 2026-10-13) | Priority P0 | Effort L | Depends: AJ-E406,AJ-E401
**Goal**: Fill all six tabs for the daily case (Instance) and for a conversation (Session), per pass2/04 §5.1-5.2, with honest empty states.
**User story**: As Persona A mid-task, I want the selected Instance's quota forecast, lineage, and actions without leaving the terminal, so that switching is a glance-and-click.
**Requirements**:
1. Instance Overview SHALL show: state+since, Provider/Account/Launch-settings chips, running model (AJ-E412), project+cwd (copyable), native session id, quota bar+forecast, tokens+cost, CPU/RAM now, children count, lineage (`from/to`) — each field following the honest-fields rule.
2. Instance Usage SHALL show token breakdown (in/out/cache-read/cache-write), tokens/min chart, cost chart (uPlot), account-window contribution %, and a confidence tag per series; Codex series SHALL be hatched + labelled `Estimated`.
3. Instance History SHALL list resumes, handovers, and crashes for this Instance with links into the Session inspector; imported history SHALL be badged `imported`.
4. Instance Actions SHALL offer Focus terminal/window (per kind), `Stop` (`Cmd/Ctrl+.`), Resume/Relaunch (ended), `Hand over...` (`Cmd/Ctrl+Shift+H`), `Queue resume at reset` (QuotaBlocked only), Export pack, Archive, Delete (guarded); irrelevant actions SHALL be disabled WITH adjacent reason text, never hidden; destructive actions SHALL use confirm popover + Undo toast.
5. Session selection SHALL render: Overview (turn count, tokens, models used, files touched), Usage (per-turn table + CSV export), History (A->B->A lineage chain), Quality/Performance (session-life tokens/min), Processes (read-only final tree snapshot from journal), Actions (Resume `Cmd/Ctrl+R` primary, Hand over, Export pack, Fork, Archive, Delete).
6. Ended Sessions SHALL be immutable with `Ended HH:MM` stamp; an unparsed transcript SHALL render `Transcript not readable yet` + parser version, never zeroed stats.
7. Refresh SHALL follow pass2/04 §5 key: event-driven < 300 ms for state/usage, 2 s resources, 30 s quota, static once ended.
**UI/UX reasoning**: NN-g error prevention (disabled-with-reason, confirm+undo for destructive) and Apple HIG keyboard support (shortcuts shown in button tooltips). Deep 02 §10/SP-04 rules govern Stop/Delete. Copy exact: `No usage recorded yet`, `No data - arrives with statusline`, `Transcript not readable yet`.
**Performance budget**: Overview paint < 100 ms; Usage chart renders 10k points < 100 ms (uPlot, EPICS M3-5 budget reused); per-turn table virtualised past 500 turns; CSV export of 10k turns < 1 s.
**Quality gates**: `just verify`; content tests per tab with populated + honest-empty fixtures; shortcut test (`Cmd/Ctrl+.`, `Cmd/Ctrl+Shift+H`, `Cmd/Ctrl+R`) with a mocked bridge.
**Evidence/validation proof**: UI tests `inspector-instance.test` + `inspector-session.test` asserting every field in reqs 1-5 by `data-testid`; pass = 100% fields present or honest-empty per fixture; live dogfood: launch fake harness, hand over, confirm lineage appears in both History tabs within 300 ms; screenshots + test log linked in PR.
**Visual testing**: Screenshots 1440x900: Instance Overview (per §7.2 wireframe), Usage (hatched Estimated series), History (lineage), Actions (disabled-with-reason + Delete confirm), Session Overview (ended, immutable stamp), Session honest-empty (unparsed transcript); Playwright diff tolerance 1%; a11y axe 0 critical, all shortcuts operable keyboard-only. Pass = reviewer matches Overview against pass2/04 §7.2 field-by-field.
**Files to touch**: `ui/src/screens/WorkPanels.tsx`, `ui/src/App.tsx`, `ui/src/components/primitives.tsx`, `ui/src/lib/bridge.ts:282,460`, `crates/juggler-app/src/commands.rs:641,722,1070`, `crates/juggler-core/src/model.rs:315,360,405`.
*Implementation mapping (corrected in this task's PR per the claiming protocol)*:
- Content lives in `RightInspector.tsx` (the E406 shell — extended, no new shell); `ui/src/lib/shortcuts.ts` (new) holds the ⌘./⌘⇧H predicates shared by App and tests; backend adds `InstanceInfo.cwd` (regenerated bindings).
- Charts (tokens/min, cost, uPlot), the per-turn table, and lineage render their honest empties with owning tasks — no time-series/pricing/Switch-Graph sources exist yet (D-06 never-hidden rule); uPlot is intentionally NOT vendored for data that does not exist.
- Live dogfood (hand-over → lineage in both History tabs) is E2-gated like lineage itself: lineage renders only from real Switch Graph edges; recorded in the manifest.
**Research basis**: pass2/04 §5.1 (lines 288-302), §5.2 (304-320), §7.2 wireframe (465-480); SPEC.md:185,288-302,347,349; deep 02 §§7,10,11.3.
**Handover note**: Lineage data depends on the Switch Graph (E2 epic); until it lands, render lineage only when edges exist and omit the section (per §5.1 empty rule) — do not fabricate A->B links from Profile history.

### AJ-E408 Inspector content for Provider, Account, and Project selections | Status: DONE-EVIDENCED (zcode-primary, 2026-10-13) | Priority P0 | Effort L | Depends: AJ-E406,AJ-E401
**Goal**: Fill all six tabs for Provider, Account, and Project selections per pass2/04 §5.3-5.5, so no selection type is a second-class citizen (D-06).
**User story**: As a developer choosing where to launch next, I want an Account's burn-down and a Project's "who is on it now" in the same inspector, so that the launch decision takes seconds.
**Requirements**:
1. Provider Overview SHALL show accounts count, running instances, aggregate quota (worst account first), capability flags, binary version + verified-range badge (`verified 2.1.x - you run 2.1.296` or drift warning).
2. Provider tabs SHALL include per-Account stacked burn (Usage, all series tier-tagged, cross-account mixes labelled `mixed`), adapter version history (History), aggregate tokens/min + error rate (Quality/Performance), all-instances process rollup (Processes, read-only), and Actions: Add account, Check for Provider update (manual only, never auto-update), Provider settings.
3. Account Overview SHALL show plan tier, login chip, quota rings (5h + weekly/credits) with `%` text, burn-down forecast + reset countdowns, running instances, usual-projects affinity, and the AJ-E411 renewal-date field; logged-out SHALL collapse Overview to the sign-in card; unknown quota SHALL render dashed rings + `No data`.
4. Account tabs SHALL include window timeline + per-session breakdown + CSV export (Usage), sessions grouped Today/Yesterday/Earlier + handovers in/out (History), tokens/min + cost-rate trends + peak footprint per Launch-settings profile (Quality/Performance), instance-tree rollup (Processes), Actions: Launch session, Sign in/Re-check login, New launch-settings profile, Rename, Delete account (danger, managed-dir-kept note).
5. Project Overview SHALL show the "On it now" Instance card (full I-01..I-11), name, copyable root + Reveal, branch summary across instances, dirty total, accounts-used history, instance/session counts; Usage = cost/tokens by Account; History = timeline with lineage links; Quality/Performance = per-instance resource overlay; Processes = all project trees + orphan section (AJ-E410); Actions: Launch with another account (ranked by headroom, `Usual` star), Hand over project, Open in editor, Pin, Rename, Archive project.
6. Missing project dir SHALL render danger banner + `Relocate...`; a project with no sessions SHALL render `Start first session` CTA.
7. Every aggregate SHALL be traceable to its rows (no dashboard number the rail cannot explain).
**UI/UX reasoning**: Nielsen heuristic 6/8: worst-account-first ordering and `Usual` star exploit recognition. Apple HIG: destructive Delete keeps its managed-dir-kept explanation adjacent (deep 02 PF-08). WCAG 1.4.1: quota rings gain `%` text (fixing primitives.tsx:178 colour-only thresholds). Copy exact: `No data`, `mixed`, `Usual`.
**Performance budget**: Aggregate computation < 50 ms for 10 Accounts x 50 Instances (memoised per 30 s quota tick); Project overlay chart < 100 ms at 10k points; tab paint < 100 ms.
**Quality gates**: `just verify`; aggregate unit tests (worst-first order, `mixed` labelling when tiers differ, dashed-ring fixture); no `QuotaRing percent={0}` fabricated usage remains reachable.
**Evidence/validation proof**: UI tests `inspector-provider/account/project.test` with 2-Provider/3-Account/2-Project fixtures asserting reqs 1, 3, 5 field-by-field; pass = all assertions green; live dogfood: log out one fixture account, confirm sign-in-card collapse; evidence (test log + screenshots) linked in PR.
**Visual testing**: Screenshots 1440x900 + 1280x800: Provider Overview (drift warning state), Account Overview (rings with % text + forecast), Account logged-out collapse, Project Overview ("On it now" card), Project missing-dir banner; Playwright diff tolerance 1%; a11y axe 0 critical + ring `role="img"` labels read quota text. Pass = reviewer verifies every number traces to a fixture row.
**Files to touch**: `ui/src/screens/WorkPanels.tsx`, `ui/src/components/primitives.tsx:165`, `ui/src/App.tsx:513`, `ui/src/lib/bridge.ts:142,149,257,296`, `crates/juggler-app/src/commands.rs:236,249,554,1203`, `crates/juggler-core/src/model.rs:120,371`.
*Implementation mapping (corrected in this task's PR per the claiming protocol)*:
- Content extends the E406 shell (`RightInspector.tsx`); backend adds `ProjectInfo.missing` (root stat — the danger-banner source); `QuotaRing` gains `%`-left text + `role="img"` (WCAG 1.4.1 gate; ring `percent` semantics = fill/used).
- Binary version / verified-range / capability flags / burn-down forecast / window timeline / adapter history render honest `No data` badges naming their wiring tasks — no fabricated vendor data. Provider "Check for update" disabled with the never-auto-update reason (SPEC §5.4 guardrail). D-10 honoured: `New launch settings…`.
- Aggregate = per-account rows worst-first (each traceable); the cross-account `mixed` label appears exactly when confidence tiers differ. Project launch ranking = quota headroom with the `Usual` star (from accounts_used).
**Research basis**: pass2/04 §5.3 (322-341), §5.4 (343-364), §5.5 (366-381); SPEC.md:146,158,257,349,358,409; deep 02 PF-03/PF-08/PP-02/PP-05/PP-07.
**Handover note**: Trap: D-10 — "New profile" in Account Actions is legacy wording; label it `New launch settings...`. Provider "Check for update" must never auto-update the binary (SPEC.md:358 guardrail test).

### AJ-E409 Quality signals without a score: tokens/min, errors, context, stuck evidence | Status: DONE-EVIDENCED (zcode-primary, 2026-10-13) | Priority P0 | Effort L | Depends: AJ-E406,AJ-E405
**Goal**: Implement the Quality/Performance tab data per D-07 — individually labelled signals, numerator/denominator shown, no composite 0-100 score anywhere.
**User story**: As Persona A deciding whether a session is healthy, I want `2 errors / 41 turns` and the stuck rule that tripped, so that I can judge quality myself instead of trusting an invented score.
**Requirements**:
1. Tokens/min SHALL be output+input tokens from transcript usage events in the trailing 5 min, rendered as 1-min bars + 5-min average; idle SHALL render real `0 tok/min`, distinguished from `no data` (no transcript yet).
2. Cost/min and session cost SHALL be tokens x a versioned in-repo pricing table, always labelled `Est - value-equiv.`; models without pricing SHALL show tokens only, never `$0.00`.
3. Error rate SHALL render as `N errors / M turns` (failed turns + api_retry/tool-error events over the trailing session or 1 h), never a bare %; Providers without a structured feed SHALL render `Not measurable (interactive)` or the Provider-specific reason.
4. Context % SHALL render only where the Provider reports context tokens (e.g. Claude statusline); elsewhere the gauge SHALL be replaced by `Context: not reported by {Provider}` + cumulative session tokens labelled `cumulative, not context`. Estimating context from cumulative tokens is PROHIBITED.
5. Stuck SHALL render the AJ-E405 rule + inputs (`no output 6m - CPU 0.3% - last write 6m ago`) with the permanent label `Stuck?`; a 0-100 stuck/health/quality score SHALL NOT exist in type, payload, or UI.
6. CPU/RAM SHALL be process-tree RSS sum + CPU% (AJ-E410 collector), labelled `Measured`, with footnote `2s sampling` on the tab.
7. Quota forecast SHALL render a range (`~30-50m at current rate`) tier-tagged; idle SHALL render `not burning`, never infinity or a point estimate.
8. Every signal SHALL carry its source + honesty tier (`Measured|Computed|Estimated`) adjacent to the value (pass2/04 §6).
**UI/UX reasoning**: NN-g trust/transparency and prior-report anti-fabrication rule (deep 01 B-14): ranges and denominators communicate uncertainty better than false precision. WCAG 1.4.1: tier is text, hatch is redundant encoding. Copy exact: `Not measurable (interactive)`, `Context: not reported by {Provider}`, `not burning`, `Est - value-equiv.`.
**Performance budget**: Signal recomputation 15 s (tokens) / 30 s (errors, forecast); per-Instance compute < 5 ms at 100k transcript events (incremental, no full re-parse); chart repaint 5 s; pricing-table lookup O(1) per model.
**Quality gates**: `just verify`; a grep-gate test SHALL fail if `quality_score|health_score|QualityScore` appears in crates/ui; pricing-table version bump requires a changelog line.
**Evidence/validation proof**: Rust tests `quality_signals` over committed transcript fixtures (Claude usage events, Codex token_usage_record, ZCode model_usage, error-pattern file, no-context Provider): assert exact tokens/min, `2 errors / 41 turns` string data, context-unavailable reason; pass = all exact-match; evidence stored as test names + fixture paths in PR. Live dogfood: open a real Claude session, confirm tokens/min moves within 15 s of a turn.
**Visual testing**: Screenshots 1440x900: Quality/Performance tab fully populated, context-unavailable variant, `Not measurable (interactive)` variant, idle `0 tok/min` vs `no data` pair; Playwright diff tolerance 1%; a11y: every chart has a text table alternative (toggle `View as table`), tier tags screen-reader visible. Pass = reviewer confirms no score-like number exists in any screenshot.
**Files to touch**: `crates/juggler-core/src/model.rs:263,279,288,299`, `crates/juggler-adapters/src/claude_transcript.rs`, `crates/juggler-app/src/commands.rs:751`, `ui/src/screens/WorkPanels.tsx`, `ui/src/components/primitives.tsx`, `docs/SPEC.md:184-235,257` (read only).
**Research basis**: decision-log D-07 (lines 70-77); pass2/04 §6 table (lines 413-421); SPEC.md:192-210,220; PROPOSAL.md:146 (>= 90% corpus claim is not a per-session probability).
**Handover note**: The pricing table does not exist today (full review F1.2: no USD view anywhere) — create it as data (model, input/output per-1k, version, source URL, date), not code constants. Trap: Codex quota is Estimated even though its tokens are Measured — keep the two tiers separate per series.
*Implementation mapping (corrected in this task's PR per the claiming protocol)*:
- Engine lives in `crates/juggler-core/src/quality/` (`signals.rs` + `pricing.rs` + `pricing.v1.json` data file) — new module, not `model.rs` edits; surfaced via the `instance_quality` command (`commands.rs`) → bridge `instanceQuality` → `QualityTab` in `RightInspector.tsx` (the E406/E407 shell, not `WorkPanels.tsx`).
- Req.2: session cost shipped (table v1: claude-code opus/sonnet, source+captured date, CHANGELOG line); cost/min joins with the AJ-E606 usage engine (needs burn deltas) — tab shows the honest owner note, no fabricated series.
- Req.5: stuck = the AJ-E405 attention rule id + inputs + `since` (schema v6 `attention_json`); grep gate test enforces no `quality_score|health_score|QualityScore` in `crates/` or `ui/src`.
- Req.7: `quota_forecast_label(quota_used_pct, pct_burned_last_5_min)` renders `~L-Hm at current rate · Est` (±33% range) or `not burning`; the command passes burn 0.0 today (deltas unobservable until AJ-E606+) — honest idle, wired for the future feed.
- Req.6: CPU/RAM footnote names the AJ-E410 collector; fixtures are provider-shape-agnostic JSONL (`tests/fixtures/`), with Codex `token_usage_record` / ZCode `model_usage` shapes deferred to their adapter tasks (E5/E6) rather than guessed.

### AJ-E410 Processes tab, resource collector, and orphan (left-behind) contract | Status: DONE-EVIDENCED (zcode-primary, 2026-10-13) | Priority P0 | Effort L | Depends: AJ-E406,AJ-E401
**Goal**: Collect per-tree CPU/RAM (I-07 sparkline source), render the live Processes tree for every selection, and surface orphaned descendants after Instance exit with verified stop actions.
**User story**: As a developer whose dev server outlived its agent, I want `Orphaned - left behind (2)` with PIDs and a verified Stop, so that zombies stop eating my machine (expected-worfklow.md:17-18).
**Requirements**:
1. A sysinfo collector SHALL sample each supervised tree every 2 s (staggered, SPEC.md:231), attributing RSS/CPU by ppid-map via `descendants()`; overhead SHALL stay < 1% CPU at 20 Instances.
2. The Processes tab SHALL render PID, name, CPU%, RSS, age, child count for Instance/Project/Provider selections (rollups read-only above Instance level), refreshing at 5 s; tree-read failure SHALL render banner + `Retry` (manual), never an empty table.
3. On Instance end (any state except clean Stopped), the descendant set SHALL be snapshotted; survivors after a 10 s grace SHALL appear as `Orphaned - left behind (N)` with PID, name, RSS, age-since-parent-exit, parent label.
4. Orphan actions SHALL be `Verify owner` (re-check ppid chain, manual), `Stop process` (SIGTERM->SIGKILL ladder, confirm popover naming the PID), `Stop all orphans` (confirm listing all PIDs); the Processes tab itself SHALL NOT kill — Actions owns kills except these orphan controls.
5. Auto-cleanup SHALL remain opt-in per Account, off by default, 60 s grace, and SHALL only ever touch PIDs the journal recorded as descendants — name-based kills are PROHIBITED (PROPOSAL.md:47); D-08 audit-logging applies to every kill.
6. After any stop, the row SHALL re-read at 2 s and show `gone` (check) or `still alive - escalated to SIGKILL`; failure keeps the row with error text.
7. Desktop-kind/adopted orphans without a tree SHALL render `resources unavailable` for I-07, never flat zero.
**UI/UX reasoning**: Nielsen/NN-g error prevention + visibility: PIDs named in confirms, post-kill verification visible. Pass2/04 §7.3 wireframe is the layout contract. Copy exact: `No subprocesses`, `No orphaned processes` (good empty state), `Orphaned - left behind (N)`, `still alive - escalated to SIGKILL`.
**Performance budget**: Collector < 1% CPU, < 30 MB RSS at 20 Instances/500 processes; sparkline data retained 5 min ring (150 samples/tree); Processes paint < 100 ms at 500 rows (virtualised).
**Quality gates**: `just verify`; guardrail test: collector/kill path refuses a PID not in the journal descendant set; stop-ladder test with fake harness spawning a child that ignores SIGTERM.
**Evidence/validation proof**: Integration test: fake harness spawns `sleep`/dev-server child, harness exits, test asserts orphan appears <= 10 s + grace, `Stop process` yields `gone` on re-read; command `cargo test -p juggler-supervisor orphan` (or repo-named equivalent); pass = orphan detected, verified gone, and an unrelated PID fixture is refused; evidence in `docs/qa/e4-orphan-fixture.md`. Live dogfood: repeat with a real `node` dev server child.
**Visual testing**: Screenshots 1440x900 per §7.3 wireframe: live tree (3 levels), orphan section (2 orphans), Stop confirm popover (PID named), `gone` verification state, good empty state; Playwright diff tolerance 1%; a11y: table has proper headers, Stop buttons labelled with PID (`Stop process 39871`). Pass = reviewer matches wireframe element-by-element.
**Files to touch**: `crates/juggler-supervisor/src/supervisor.rs:341,435,557`, `crates/juggler-supervisor/src/journal.rs:22,151`, `crates/juggler-supervisor/src/lifecycle.rs`, `crates/juggler-app/src/commands.rs:503,733`, `ui/src/screens/WorkPanels.tsx`, `docs/SPEC.md:225,231-235` (read only).
**Research basis**: pass2/04 §5.1 Processes row + orphan contract (lines 301, 383-403), §7.3 (482-494); SPEC.md:225,234; expected-worfklow.md:15-18; decision-log D-08 (notify-only default, Clean-safe opt-in).
**Handover note**: Zero setpgid/killpg exists under crates/ today (pass2 05 §2) — attribution is ppid-map + journal, not process groups; do not claim tree-kill safety beyond recorded descendants. Coordinate PID-identity verification (pid reuse) with the E5 cleanup epic before wiring `Stop all`.
*Implementation mapping (corrected in this task's PR per the claiming protocol)*:
- Engine: new `crates/juggler-supervisor/src/orphans.rs` (tracker + TERM→KILL ladder + `verify_owner`) + `LiveStatsCollector` upgraded to the 2 s tick model (per-tree `ProcessRow`s + 150-sample rings); journal gains `descendants` (durable allow-list) and `killed` (D-08 audit) entries — `supervision.jsonl` stays append-only, old readers skip new tags.
- Wiring: `processes_tick` (2 s) joins the read-model ticks; the lifecycle forwarder calls `record_exit_snapshot` on every Exited (clean exits recorded, never surfaced); commands `instance_processes` / `orphan_stop` / `orphan_stop_all` / `orphan_verify` / `account_set_cleanup` registered; `Account.cleanup_opt_in` lands via schema v7 (additive, default 0).
- Staggering: one sampler thread + one sysinfo table means the per-tree herd cannot exist by construction — targeted refreshes of known pids every 2 s, full-table discovery every 5th tick (+3 warmup ticks); overhead gate measures targeted-tick mean at 20 trees against the 1%-of-2s budget. Req.2's `descendants()` is realised as the ppid-map walk inside the collector (the old `Supervisor::descendants` one-shot helper remains for compatibility).
- UI: `ProcessesTab` (live tree + orphans + confirms) and read-only `AggregateProcesses` rollups in the E406 shell; the tab's data source is the injectable `ProcessesBridge` seam (default = real bridge) because vitest 5's mocker did not intercept this panel's dynamic-import site; Quality/Performance gains `CPU / RAM now` (Measured) + `2s sampling` footnote — completing E409 Req.6.
- `Stop all` ships guarded by the recorded-descendant allow-list only; PID-identity verification (pid reuse, port re-probe) stays the E5/AJ-E503 seam per the handover note — no tree-kill safety claimed beyond recorded descendants.
- Fixtures: fake-harness gains `--spawn-child PROG --child-args "…" [--child-ignore-term]` (stdio null; the ignore-term variant wraps in `bash -c "trap '' TERM; exec …"` — SIG_IGN survives exec; workspace forbids unsafe code so no pre_exec).

### AJ-E411 Subscription-expiry warning hook (manual renewal date, D-12) | Status: DONE-EVIDENCED (zcode-primary, 2026-10-14) | Priority P1 | Effort M | Depends: AJ-E401,AJ-E408
**Goal**: Add the optional user-entered renewal date per Account, its approaching-expiry warning on row + Account inspector, and the gated research hook for any future sanctioned source.
**User story**: As Persona A juggling three subscriptions, I want `Renews in 6 days` on the Account that is about to lapse, so that I plan the switch before the wall, not after.
**Requirements**:
1. `Account` SHALL gain optional `renewal_date` (calendar date, user-entered only); migration SHALL be additive and serialisation-compatible with existing stores (D-02 phased rename discipline).
2. With no date entered, expiry SHALL render `Expiry: unknown` on the Account inspector and SHALL NOT appear on the row; inference or guessing from plan tier, usage, or billing cadence is PROHIBITED (D-12).
3. Warning thresholds SHALL be: > 14 days = date only (`Renews 2026-11-02`); 8-14 days = `Renews in N days`; <= 7 days = warning chip `Renews in N days` (amber); <= 0 = danger chip `Renewal date passed` — all as icon+text, never colour-only.
4. The Account inspector Overview SHALL host the editor (`Renewal date (optional)` date input + `Clear`), with helper text `Entered by you - Juggler does not read billing`; editing SHALL be available from Account Actions as `Edit renewal date...`.
5. The indicator SHALL participate in AJ-E404 Needs-me ranking only at <= 7 days and only as a tie-breaker below Failed/QuotaBlocked/WaitingUser/Stuck — expiry never outranks a live blocked agent.
6. A research hook SHALL exist as a documented seam (trait/function returning `Unknown` today) plus the mandatory D-12 research note: any future Provider billing/renewal source must be sanctioned, read-only, and verified live before it may populate this field; unproven sources stay `unknown`.
7. Dates SHALL be stored as local calendar dates (no timezone conversion); a past date entered today SHALL warn immediately, not be "corrected".
**UI/UX reasoning**: Apple HIG (clear, user-owned data) + NN-g honesty: the "entered by you" helper prevents false authority. WCAG 1.4.1/4.1.3: threshold states are text chips; crossing <= 7 days is announced politely once per day, not on every tick. Copy exact: `Expiry: unknown`, `Renewal date passed`, `Entered by you - Juggler does not read billing`.
**Performance budget**: Zero new polling — expiry computed on the existing 15 s UI tick and on date edit; storage migration < 100 ms at 100 Accounts; warning evaluation < 1 ms/Account.
**Quality gates**: `just verify`; migration round-trip test (store without the field loads as `None`); threshold boundary tests at 14/8/7/1/0/-1 days with a fixed clock.
**Evidence/validation proof**: Command `cargo test --workspace renewal` (or repo-named equivalent) asserting the six boundary cases + no-date `unknown`; pass = 7/7; live dogfood: set a date 5 days out in the dev app, confirm amber chip on Account inspector and no row badge, then set yesterday and confirm `Renewal date passed`; record in PR with screenshots (below). Research deliverable: `docs/research/subscription-expiry-sources.md` stub stating the D-12 gate, created or explicitly deferred with owner named.
**Visual testing**: Screenshots 1440x900: Account Overview with each threshold state (> 14d, 8-14d, <= 7d amber, passed danger), `Expiry: unknown` state, editor open with helper text; viewports also 1280x800 for editor; Playwright diff tolerance 1%; a11y: date input labelled, chips readable by screen reader as full text. Pass = reviewer confirms no inferred date appears anywhere.
**Files to touch**: `crates/juggler-core/src/model.rs:145-157`, `crates/juggler-app/src/commands.rs:249,265,310`, `ui/src/types/generated/AccountInfo.ts` (generated), `ui/src/screens/WorkPanels.tsx`, `ui/src/lib/bridge.ts:149,156`, storage migration in `crates/juggler-app/src/state.rs:34` (or repo storage layer it calls).
**Research basis**: decision-log D-12 (lines 113-124, incl. mandatory billing-API research); pass2 00 §1.2 item 11 (plan/subscription expiry was ownerless — this task owns it); expected-worfklow.md:3 (quota/subscription expiry motivation).
**Handover note**: Do NOT scrape Provider dashboards or infer from `PlanTier` (model.rs:120) — D-12 explicitly forbids it and pass2 proved no verified source. If the research finds a sanctioned API, it is a NEW task; this hook only reserves the seam.
*Implementation mapping (corrected in this task's PR per the claiming protocol)*:
- Engine: new `crates/juggler-core/src/renewal.rs` — `CalendarDate` (strict leap-aware `YYYY-MM-DD` parsing), `renewal_label` (pure, fixed-clock threshold tests), `is_row_visible`, and the D-12 `RenewalSource` seam (only `UnknownRenewalSource` ships). `Account.renewal_date: Option<String>` via schema v8 (additive nullable column).
- Storage lives in juggler-core's repo layer (not the app state file the task text guessed); the command surface is `account_set_renewal` (validation via `AppError::Validation`, first user-input validation surface) + `AccountInfo.renewal_date` on the wire.
- Threshold labels are computed in TS (`ui/src/lib/renewal.ts`) against the machine's LOCAL calendar date (Req.7's local-date rule is exact in JS; the Rust fn is the canonical seam + research hook and both boundary suites assert identical copy — they change together).
- Needs-me tie-break threads App → WorkflowRail → InstanceBoard → `sortSessions(…, accounts)`; expiry ranks only after `severityRank || snoozed || waiting`, so it never outranks a live blocked agent.
- The once-per-day polite announcement ships as `RenewalAnnouncer` (aria-live polite, localStorage last-announced date per account).
- Research deliverable: `docs/research/subscription-expiry-sources.md` — the D-12 gate, candidate audit (all unsanctioned/prohibited), and the new-task revisit trigger.

### AJ-E412 Running-model telemetry (what the Instance is actually running) | Status: DONE-EVIDENCED (zcode-primary, 2026-10-14) | Priority P1 | Effort M | Depends: AJ-E401,AJ-E407
**Goal**: Report the model an Instance is actually running now — launch overlay, in-session `/model` changes, and transcript-observed model — with source labelling and drift surfacing.
**User story**: As Persona A who switched models mid-session, I want the row to show the model in use right now (not what I launched with), so that cost and quota judgements use the truth.
**Requirements**:
1. The read model SHALL distinguish `launch_model` (from `SettingsOverlay.model`, may be null) from `running_model` (nullable, with `model_source` in `launch|in-session|transcript|unknown`); I-03 SHALL render `running_model` once known, else the launch model, else `default model`.
2. In-session model changes SHALL be detected from the transcript/statusline watch (SPEC.md:190) and emitted over `sessions_updated` within 300 ms; no polling of Provider config files is permitted.
3. When `running_model` differs from `launch_model`, the Instance Overview SHALL show both (`Launched with opus-4.8 - now running sonnet-5`) and I-03 SHALL gain a drift marker with that text as its tooltip/`aria-label`.
4. Cost signals (AJ-E409) SHALL price usage by the model observed per turn where the transcript records it; turns without a recorded model SHALL be priced as `unknown model` and excluded from $ totals (tokens still counted).
5. Providers with no observable running model SHALL render `Model: not reported by {Provider}` in the inspector (row keeps `default model` only when no launch model exists, else launch model + `not verified live` tag).
6. Model identifiers SHALL be displayed verbatim (vendor strings, e.g. `opus-4.8`, `gpt-5-codex`); normalisation/mapping tables are PROHIBITED except a display-only family grouping in filters.
7. Telemetry SHALL be local-only (SPEC privacy: telemetry opt-in only, SPEC.md:23,353): running-model data never leaves the machine except over the D-13 Fleet link to paired machines.
**UI/UX reasoning**: Nielsen/NN-g visibility + honesty tiers applied to identity data: the `not verified live` tag prevents launch intent masquerading as fact (same pattern as quota tiers, pass2/04 §4.1). WCAG 1.3.1: drift is text, not just a dot. Copy exact: `default model`, `Model: not reported by {Provider}`, `not verified live`, `Launched with X - now running Y`.
**Performance budget**: Model detection adds < 1 ms per transcript event (single field read on already-parsed lines); no extra file watchers beyond the existing transcript watch; row update within the 300 ms event budget at 50 Instances.
**Quality gates**: `just verify`; parser fixtures: launch-only, `/model` change mid-transcript, model-per-turn mixed session, no-model Provider; guardrail: no new network/file polling introduced (code review checklist item).
**Evidence/validation proof**: Rust/UI tests asserting I-03 value + `model_source` for the four fixtures (exact strings); pass = 4/4; live dogfood: in a real Claude session run `/model`, confirm row + Overview update <= 300 ms (screen recording or timestamped screenshots in PR); evidence linked, fixtures committed beside `crates/juggler-adapters/src/claude_transcript.rs` tests.
**Visual testing**: Screenshots 1440x900: I-03 plain (launch==running), I-03 with drift marker + tooltip open, Instance Overview dual-model line, `Model: not reported by {Provider}` inspector state; Playwright diff tolerance 1%; a11y: drift announced as text, tooltip reachable by keyboard focus. Pass = reviewer can state launch vs running model from screenshots alone.
**Files to touch**: `crates/juggler-core/src/model.rs:161-168,299-321`, `crates/juggler-adapters/src/claude_transcript.rs`, `crates/juggler-app/src/commands.rs:911`, `ui/src/types/generated/SessionInfo.ts` (generated, via AJ-E401), `ui/src/components/WorkflowRail.tsx:397`, `ui/src/screens/WorkPanels.tsx`, `docs/SPEC.md:190` (read only).
**Research basis**: pass2/04 §3 I-03 (line 177: launch + `/model`-change via transcript watch, SPEC.md:190); pass2/04 §6 cost row (416); decision-log D-06 honest-fields rule; expected-worfklow.md:13 ("models currently running").
**Handover note**: Trap: `SettingsOverlay.model` (model.rs:164) is intent, not telemetry — never label it as running. If a Provider exposes the model only at launch flags, `model_source=launch` + `not verified live` is the honest end state, not a bug.
*Implementation mapping (corrected in this task's PR per the claiming protocol)*:
- Read model: `InstanceRecord.running: Option<RunningModel{model, source}>` (schema v9 additive JSON blob — same pattern as attention v6); wire `InstanceInfo` gains `launch_model`/`running_model`/`model_source` and `model` stays the computed display value so existing I-03 consumers hold.
- Detection (Req.2): rides the EXISTING 200 ms activity drain (event path) + the 2 s tick as safety net — incremental byte-offset transcript tail scans (seek-based; one metadata read when unchanged). No new watcher thread, no Provider config polling, no network. `detect_running_models_core` (storage) is test-separable from the emit wrapper.
- Cost (Req.4): `session_cost_per_turn` prices each turn by its observed model; unrecorded/unpriced turns are excluded from $ but kept in token totals — this REPLACES E409's launch-overlay fallback in `instance_quality` (overlay = intent, never priced as fact).
- Parser: `ParsedMessage.model` reads both shapes (top-level `model` and nested `message.model`); fixtures at `crates/juggler-adapters/tests/fixtures/model_*.jsonl` beside the parser tests.
- UI: `modelDrift` helper + keyboard-focusable `⇄` marker (aria = full drift text) in the rail; Overview renders the dual-model line, `not verified live` chip, or `Model: not reported by {Provider}`.
- Live `/model` dogfood against a real Claude session remains maintainer-side validation (same class as prior live-dogfood items); the deterministic fixture→detection→wire→UI path is fully evidenced.

### AJ-E413 Desktop adapter contract — launch and track a windowed harness | Status: DONE-EVIDENCED (evidence: docs/qa/evidence/AJ-E413/manifest.md) | Priority P0 | Effort L | Depends: AJ-E406
**Goal**: Make `InstanceKind::Desktop` real: a desktop-class harness can be launched under an Account, tracked as an Instance, and stopped, without pretending it has a terminal.
**User story**: As a user of a desktop AI app, I want Juggler to start it under the right account, show it beside my terminal instances, and stop it cleanly.
**Requirements**:
1. The adapter contract SHALL gain a desktop launch path (program/bundle id, args, env overlay, account state dir where the vendor supports it) returning PID + bundle identifier; env/account isolation limits SHALL be stated per Provider, and single-account desktop apps SHALL be labelled `Single-account app — Juggler tracks, cannot isolate` instead of implying isolation.
2. The supervisor SHALL track the desktop process in the same spawn ledger as terminal instances (E5), including children, exit, and crash state; no PTY SHALL be allocated.
3. The Instance panel SHALL render the desktop surface (status, account, project, resources, transcript/usage where readable) with `Open / Focus window` as primary action (AJ-E414); the terminal card SHALL NOT render for desktop kind.
4. Where a Provider ships a CLI engine under the desktop app (e.g. ZCode, SPEC.md:134), the adapter SHALL prefer offering the CLI engine as a terminal Instance and the desktop app as focus-only, with the choice shown to the user at Add time.
**UI/UX reasoning**: NN-g #2 match with real world: desktop instances say "app", show a window action, and never show a fake terminal. Kind badge is text (`app` chip exists in rail), never icon-only (WCAG 1.4.1).
**Performance budget**: Launch-to-tracked <3 s; status propagation <1 s from process exit; tracking adds <0.5% CPU per desktop instance.
**Quality gates**: Fixture desktop app (scriptable fake) launches, is tracked, exits, and is reconciled after Juggler restart; `just verify` green; no credential or login automation introduced.
**Evidence/validation proof**: Store fake-desktop e2e log + one real desktop harness launch record (sacrificial project) in `docs/qa/evidence/AJ-E413/`; pass = lifecycle states correct, isolation claim matches the per-Provider label.
**Visual testing**: Capture desktop Instance panel + rail `app` chip at 1280x800/1024x680, light/dark; states: running, exited, crashed, single-account warning; pass = no terminal surface, Focus is the primary action, text states readable (AJ-E803 compare).
**Files to touch**: `crates/juggler-adapter/src/adapter.rs`, `crates/juggler-adapter/src/launch.rs`, `crates/juggler-supervisor/src/supervisor.rs`, `ui/src/screens/WorkPanels.tsx:116-126` (replace placeholder), `crates/juggler-core/src/model.rs:325-355`.
**Research basis**: model.rs:330-332 Desktop kind; SPEC.md:321,332-335,347,351; ADR-0007 decisions 4-5; WorkPanels.tsx:122-123 placeholder text.
**Handover note**: True window embedding is impossible on macOS (no public reparenting API) — do not attempt it. The panel is the surface; the window is reached via AJ-E414.

*Implementation mapping (corrected in this task's PR per the claiming protocol)*:
- Adapter (Req.1): `juggler-adapter` gains `DesktopLaunch { program, bundle_id, args, env, cwd, isolation_label }` + trait `desktop_launch` (default `Unsupported`) and `desktop_isolation_label` (default `None`); capability bits `DESKTOP_APP`/`DESKTOP_SINGLE_ACCOUNT`. Reference implementation: `juggler-adapters` `FakeDesktopAdapter` + the scriptable `fake-desktop` binary (`--marker/--exit-after-secs/--crash/--spawn-child`).
- Wire (Req.1): `commands.rs` `DesktopLaunchInfo { session_id, pid, bundle_id, isolation_label }` and `InstanceInfo.isolation_label` (registry-resolved at read time; `null` for terminal rows and unregistered providers — unit-tested).
- Supervisor (Req.2): `Supervisor::spawn_desktop` — same spawn ledger, null stdio (no PTY), journal `spawned`/`exited`, kernel wait thread → `LifecycleEvent::Exited`; stop through the shared TERM→KILL ladder; crash surfaces `ExitKind::Code(n)`.
- Panel (Req.3): `RightInspector.tsx` `InstanceOverview` desktop branch — `desktop-surface` card with status/`app` chip/provider/account·project/resources/tokens, primary `Open / Focus window` (injected `focusBridge` seam), amber `desktop-isolation` callout (copy-exact label), focus-outcome tier text, exited/crashed state lines; terminal card and hand-over section NOT rendered (asserted). `desktop_focus` = opener app-activation tier with honest outcomes (`App not running — Relaunch?` / `App opened — exact-window focus lands with AJ-E414`).
- Req.4: capability-gated both ways (`ships no desktop app` refusal test; fake-desktop `launch_spec` refuses CLI). No dual-capability provider ships in the default registry yet — the Add-time dual offer renders when one does (ZCode, SPEC.md:134); the seam is the capability bits + both launch paths.
- Evidence: `docs/qa/evidence/AJ-E413/` (fake-desktop e2e log, real TextEdit launch record incl. journal lines, manifest); gallery `_gallery/13-desktop/` (6 captures, states running/exited/crashed/single-account warning, dark+light).

### AJ-E414 Focus Window service — go to that exact instance's window | Status: DONE-EVIDENCED (evidence: docs/qa/evidence/AJ-E414/manifest.md) | Priority P0 | Effort M | Depends: AJ-E413
**Goal**: One click (or hotkey) from any desktop Instance raises that application's exact window, even when the app holds several windows or sits on another Space or minimised.
**User story**: As a user juggling three desktop windows, I want "Focus window" to land me on the right window, not just the right app.
**Requirements**:
1. Focus SHALL proceed in tiers: (a) activate the running application (NSRunningApplication.activate); (b) find the target window for this Instance via the Accessibility API (AXUIElement by PID, matched by recorded window title/identity); (c) clear `AXMinimized` if set, then perform `kAXRaiseAction`; app-level activation alone SHALL NOT be reported as window focus success.
2. The Instance SHALL record its window identity at launch/first sight (owning PID, bundle id, window title, CGWindow id where available) and re-discover it on each focus attempt; stale identities SHALL be re-enumerated, never assumed.
3. If Accessibility permission is missing, Focus SHALL degrade to app activation and show a one-time, dismissible explainer: "Juggler can open the app; exact-window focus needs Accessibility permission" with a deep link to the system setting; the permission SHALL never be requested silently or bundled into an unrelated prompt.
4. A global, user-configurable hotkey and the command palette SHALL offer "Go to instance…" listing live desktop (and terminal) instances; selecting one focuses its window or its terminal panel respectively.
5. Focus outcomes SHALL be classified and surfaced honestly: `Focused window`, `App opened — window not found`, `App not running — Relaunch?`, each as text on the Instance after the attempt.
**UI/UX reasoning**: NN-g #1 visibility + #3 user control: the action reports which tier it achieved; the hotkey follows the SPEC tray/hotkey pattern (§9.2) so keyboard users never hunt Mission Control. Permission copy follows Apple HIG: explain at the moment of need, once.
**Performance budget**: Focus completes <500 ms after click on a warm app; window enumeration <100 ms for <20 windows; hotkey-to-list <100 ms.
**Quality gates**: Automated fixture (two windows, one minimised, fake app) focuses the correct window 10/10; missing-permission fixture degrades with explainer and no crash; `just verify` green.
**Evidence/validation proof**: Store fixture run + screen recording of a real two-window desktop app focus (incl. minimised and other-Space cases) in `docs/qa/evidence/AJ-E414/`; pass = correct window frontmost in all recorded cases, outcome text matches reality.
**Visual testing**: Capture Instance panel before/after focus with each outcome text, palette "Go to instance…" list, and the permission explainer at 1280x800 both themes; pass = outcome and next action unambiguous from screenshots alone (AJ-E803/E804 keyboard traversal of the palette included).
**Files to touch**: new `crates/juggler-app/src/focus.rs` (AX via accessibility-sys or objc2-app-kit — pin in PR), `crates/juggler-app/src/commands.rs`, `ui/src/screens/WorkPanels.tsx`, `ui/src/App.tsx` (hotkey), `crates/juggler-core/src/model.rs` (window identity fields).
**Research basis**: External macOS evidence gathered 2026-10-13: AXRaise + NSRunningApplication.activate pattern; minimised windows need AXMinimized cleared first (claude-notifications-go issue #67 pattern); SPEC.md:335 "focus its window"; wireframe screen 24.
**Handover note**: Traps: AXRaise on a minimised window is a silent no-op; windows on another Space may be invisible to AX until the app is activated — activate, then re-enumerate. Never focus by PID alone when titles collide; record the ambiguity as `window not found`.

*Implementation mapping (corrected in this task's PR per the claiming protocol)*:
- Mechanism pin (recorded deviation): `accessibility-sys`/`objc2-app-kit` require `unsafe` FFI, barred by the workspace `unsafe_code = "forbid"` law — the same AX operations run through `/usr/bin/osascript` + System Events, and tier (a) activation via `/usr/bin/open -b` (the `LaunchServices` equivalent of `NSRunningApplication.activate`, needing no grant at all).
- Tiers (Req.1): new `crates/juggler-app/src/focus.rs` — `FocusService::attempt` over the scriptable `ScriptRunner` seam: activate ALWAYS first (other-Space trap), enumerate by recorded PID (stale → re-discover by bundle id, persist fresh PID), match recorded title (absent → first-sight adoption; ≠1 match → `window not found`), then one atomic raise script (`set AXMinimized false` → `perform AXRaise` → `set frontmost`; clearing FIRST per the no-op trap).
- Identity (Req.2): `InstanceRecord.window: Option<WindowIdentity>` (schema v10 additive blob + `app_settings` kv table); pid+bundle recorded at `desktop_launch`; title adopted at first sight; `cgwindow_id` honestly `null` (no non-FFI source).
- Permission (Req.3): every System Events error classified by stderr (`is_permission_denied`) → degrade to activation + `FocusOutcomeInfo.permission_missing`; panel renders the one-time dismissible explainer (copy exact) + the fixed deep link (`open_accessibility_settings`). Live-caught during the real record: process-NAME probes pass WITHOUT the grant (window reads are the AX-protected surface) — classification therefore lives on the enumeration errors, not an upfront probe.
- Hotkey + palette (Req.4): `tauri-plugin-global-shortcut` (stored binding, default `Alt+Space`; `go_to_hotkey_set` registers first, restores on failure) emitting `app://go-to`; the "Go to instance…" palette (⌘K or hotkey) — live instances, desktop first, combobox a11y, Enter → window focus / panel open, in-palette rebind capture. The full §4.17 palette remains its own unowned surface.
- Outcomes (Req.5): copy-exact `Focused window` / `App opened — window not found` / `App not running — Relaunch?` as text on the Instance (+ targeted window title).
- Evidence: `docs/qa/evidence/AJ-E414/` — 12/12 tier unit tests (two-window fixture 10/10 with trap-order asserts), 9/9 desktop integration, real two-window TextEdit record (this machine: degraded tier 10/10 in 118–189 ms; the granted-machine focused record awaits the maintainer's Accessibility toggle — same command re-run); gallery `_gallery/14-focus/` (4 captures, vision-verified).

### AJ-E415 Desktop live preview — read-only window mirror in the panel | Status: DONE-EVIDENCED (evidence: docs/qa/evidence/AJ-E415/manifest.md — complete via D-17, 2026-10-13) | Priority P1 | Effort L | Depends: AJ-E413, AJ-E414
**Goal**: Show a live, read-only ScreenCaptureKit preview of a desktop instance's window inside its panel, so its state is glanceable without switching away; clicking the preview focuses the real window.
**User story**: As a user supervising a desktop agent, I want to see whether it is working, stuck or asking something, without leaving Juggler.
**Requirements**:
1. The preview SHALL use ScreenCaptureKit per-window capture (`SCContentFilter` desktop-independent window) rendered in the desktop Instance panel at a capped frame rate (≤15 fps, ≤1280 px wide) and SHALL pause when the panel is not visible or the window is minimised/closed, showing `Preview paused — window not visible` as text.
2. The preview SHALL be strictly read-only: no input injection, no synthetic clicks/keys into the captured app, anywhere in this task; the only pointer action is click-to-focus (AJ-E414).
3. Screen-recording permission SHALL be requested only when the user first enables preview, with an explainer and a permanent off switch per Account; without permission the panel shows status/transcript data only.
4. Capture SHALL exclude Juggler's own windows and SHALL stop within 1 s of panel close, instance exit, or permission revocation; a visible `● Live preview` indicator with the frame age SHALL be present while streaming.
5. CPU/GPU cost SHALL be budgeted: one preview at a time by default (the selected instance), additional previews require an explicit user pin and show their cost in Settings.
**UI/UX reasoning**: NN-g #1 visibility without context-switch cost; honesty rules (D-06): a stale frame is labelled with its age, never presented as live. Click-to-focus keeps the real app one gesture away (progressive disclosure).
**Performance budget**: Preview adds <5% CPU on reference Mac at cap; frame latency <300 ms; stream teardown <1 s; zero cost when no desktop panel is open.
**Quality gates**: Fixture: preview renders a changing fake-app window, pauses on minimise, stops on close (resource log proves teardown); permission-denied path renders the no-permission panel state; `just verify` green.
**Evidence/validation proof**: Store resource-usage log (CPU before/during/after) + recordings of live/paused/denied states in `docs/qa/evidence/AJ-E415/`; pass = budgets met, no stream survives panel close in three consecutive runs.
**Visual testing**: Capture live, paused (minimised), permission-denied and instance-exited panel states at 1280x800/1024x680 both themes; pass = live indicator + frame age readable, click-to-focus affordance obvious, no interactive-looking fake controls inside the preview (AJ-E803).
**Files to touch**: new `crates/juggler-app/src/preview.rs` (ScreenCaptureKit via screencapturekit crate or objc2 — pin in PR), `ui/src/screens/WorkPanels.tsx`, `ui/src/App.tsx` (panel lifecycle), Settings surface (per-Account preview toggle).
**Research basis**: External macOS evidence 2026-10-13: macOS does not permit reparenting a foreign NSWindow (window server isolation); ScreenCaptureKit per-window streams are the sanctioned mirror (one SCStream per window); SPEC.md:57-59 panel-as-surface; wireframe screens 08/24 pattern.
**BLOCKED record (2026-10-14)**: the safe subset landed (schema v11 opt-in, driver/state machine + 8/8 scripted suites incl. teardown ×3 and revocation, wire events, panel states live/paused/denied/exited, account toggle, 5 captures, CLT Swift-link shim) — but the binding (`screencapturekit` v11) requires `unsafe { sc_initialize_core_graphics() }` at process start or the FIRST REAL CAPTURE SEGFAULTS (reproduced; log in evidence). The shipped app wires `ParkedSource` (honest unsupported state — no crash, no fake frame). Unblock options recorded in the manifest's decision section; then re-point at `ScreenshotSource`, re-run `tests/preview_real.rs`, and flip DONE-EVIDENCED.

**Handover note**: Do not let preview grow input injection ("just one click-through") — that converts a supervision surface into remote control of another app, a different security decision requiring its own ADR. If the screencapturekit binding proves immature, ship AJ-E413/AJ-E414 and mark this task BLOCKED with the binding gap recorded, never hand-roll private APIs.

---

## Appendix A — Sequencing and dependency rationale (read before starting any task)

The order below is a topological order of the `Depends:` fields; it is also
the merge order. Do not merge a rendering task before its data contract.

| Wave | Tasks | Why this wave exists |
|---|---|---|
| W0 | AJ-E401 | Every indicator and inspector field needs a typed, nullable payload first. |
| W1 | AJ-E402, AJ-E405 | The row can render honest empties without emission; emission fills them live. |
| W2 | AJ-E403, AJ-E404, AJ-E406 | Density/ranking are row-local; the inspector shell is independent of content. |
| W3 | AJ-E407, AJ-E408, AJ-E410 | Content tabs and the process collector fill the shell. |
| W4 | AJ-E409, AJ-E412 | Quality signals and running-model telemetry refine Usage/Performance/I-03. |
| W5 | AJ-E411 | Expiry is self-contained P1; it must not block the P0 daily surface. |

Cross-epic dependencies the next agent must respect:

- E2 (Switch Graph) owns lineage edges; AJ-E407 renders them, never invents them.
- E5 (Cleanup) owns PID-identity verification and Clean-safe tiers (D-08);
  AJ-E410 renders orphans and verified stops inside that contract only.
- M3 usage engine (EPICS.md:86-95) owns quota feeds and burn-down math;
  E4 renders I-05/I-06 and Account/Provider aggregates from it, labelled.
- M2-5 (EPICS.md:79) owns the stuck-fixture corpus; AJ-E405 wires its
  heuristics into live emission and owns the >= 90% precision/recall claim.
- D-13 fleet: `machine` appears in AJ-E401 payload, AJ-E404 keys, and
  AJ-E406 remote rendering now, even though Fleet ships later (M6).

Definition of "unblocked": a task is unblocked when each `Depends:` task
has merged WITH its evidence artifact (test log, fixtures, screenshots),
not merely with code. A dependency merged without evidence does not count.

## Appendix B — Shared contracts every E4 task must reuse

1. **Honest-fields rule (D-06).** Absence renders as `unavailable` + reason
   (`Quota: no data`, `Context: not reported by {Provider}`,
   `Not measurable (interactive)`, `Expiry: unknown`). Fabricated zeros,
   dressed-up estimates, and hidden fields are review-blocking defects.
2. **Confidence tiers.** `Measured | Computed | Estimated`
   (`model.rs:288`) appears as text adjacent to every usage/quota/cost
   number. Hatch/colour is redundant encoding only (WCAG 1.4.1).
3. **Stale rule.** Value older than 2x its refresh interval renders dimmed
   with `stale - Nm`; it never renders as fresh (AJ-E403 owns the rule,
   AJ-E406 applies it per inspector field with a `source:` footer).
4. **Refresh key.** E = event-driven < 300 ms; resources 2 s sample / 5 s
   repaint; activity + waiting timers 15 s; quota + errors + forecast 30 s;
   countdown text 1 min. New intervals require a SPEC citation.
5. **State vocabulary.** Exactly the 9 `SessionStatus` wire values
   (`model.rs:230-238`); unknown wire values render `Unknown`. The legend
   glyphs/text in pass2/04 §4.1 are the only approved renderings.
6. **No composite score (D-07).** `quality_score`, `health_score`, or any
   0-100 rollup in type, payload, or UI is prohibited (AJ-E409 grep-gate).
7. **Passive automation (SPEC §7, D-08).** E4 surfaces suggest, rank, and
   warn; kills/launches/handovers execute only on explicit user click,
   except per-Account Clean-safe opt-ins owned by the E5 epic.
8. **Terminology (D-02/D-10).** Provider / Account / Instance / Session /
   Project in all copy; `Profile` only as "Launch settings"; the word
   "harness" never appears in user-visible strings.

## Appendix C — Fixture and evidence inventory (minimum set, all committed)

Backend fixtures (extend the fake harness, `crates/juggler-adapters/src/fake.rs`):

- `fake:steady` — runs, emits tokens every 5 s, exits 0. Feeds AJ-E401/E402/E409.
- `fake:hang` — emits once, then no output, CPU near 0. Feeds AJ-E405/E409.
- `fake:wait-permission` — prints a permission prompt, blocks. Feeds AJ-E405.
- `fake:quota-exit` — exits non-zero with a rate-limit signature. Feeds AJ-E405.
- `fake:crash` — exits non-zero without a signature. Feeds AJ-E405/E407 History.
- `fake:spawner` — launches a child that survives parent exit. Feeds AJ-E410.
- `fake:model-switch` — transcript records a mid-session model change. Feeds AJ-E412.
- Transcript files: Claude usage events, Codex `token_usage_record`, ZCode
  `model_usage`, an error-pattern file, and a no-context-Provider file (AJ-E409).

UI fixtures (bridge mock, `ui/src/lib/bridge.ts:65` MOCK_SESSIONS pattern):

- One row per `SessionStatus` (9), one no-quota row, one no-pricing row,
  one non-git row, one 0-children row, one unknown-status row (AJ-E402).
- An 8-row severity ladder + 1 snoozed + 1 no-quota row (AJ-E404).
- One selection per type (Instance/Session/Provider/Account/Project) with
  populated and honest-empty variants of every tab (AJ-E406/E407/E408).
- Account dates at +30/+10/+5/0/-1 days from a fixed clock (AJ-E411).

Evidence storage convention: test logs and fixture reports go in the PR
description; gate-level summaries go under `docs/qa/` (e.g.
`docs/qa/e4-attention-fixtures.md`, `docs/qa/e4-orphan-fixture.md`);
approved screenshot baselines live beside the Playwright suite that
produced them. An evidence link that requires the author's machine is not
evidence — it must be reachable from the repository or CI run.

## Appendix D — Epic gate E4-G (all must hold before E4 is Done)

1. `just verify` green on the merge commit (fmt, clippy `-D warnings`,
   workspace tests, docs) — QUALITY.md Definition of Done, no exceptions.
2. AJ-E405 fixture corpus: stuck/waiting/quota classification >= 90%
   precision AND recall, report committed under `docs/qa/`.
3. No fabricated values reachable: grep + screenshot audit finds no
   `percent={0}` quota, no `$0.00` unknown-pricing cost, no quality score.
4. Visual suite green: every AJ-E402-E412 screenshot set captured at its
   named viewports, Playwright diff <= 1% vs approved baselines, axe-core
   0 critical/serious on rail + inspector fixture pages.
5. Performance budgets re-measured at 50 Instances / 200 rows: sort <
   16 ms, selection paint < 100 ms, collector < 1% CPU, payload < 8 KB/row.
6. Live dogfood log (one page, `docs/qa/`): a real day-shape run — at
   least one WaitingUser answered from rank 1, one `/model` switch seen
   live (AJ-E412), one orphan stopped and verified gone (AJ-E410), one
   renewal date crossing the <= 7 day warning (AJ-E411, may be staged
   with a near date). Each entry names the Instance/Account fixture used.
7. Honest-fields audit: a reviewer walks all 30 tab/selection combinations
   (6 tabs x 5 types) and confirms each field is measured, labelled, or
   `unavailable` + reason. The completed checklist is the gate artifact.

## Appendix E — Traps index (the five fastest ways to fail E4 review)

- Rendering a number the backend labelled absent (null -> 0). See AJ-E401.
- Colour-only state, quota, or expiry meaning. See AJ-E402/AJ-E403/AJ-E411.
- Treating launch intent as live fact (model, quota, login). See AJ-E412.
- Hiding an unmeasurable field instead of naming why. See AJ-E406/AJ-E409.
- Killing by process name or unrecorded PID. See AJ-E410; D-08 audit applies.

## Appendix F — Indicator field contract (I-01..I-11, normative summary)

This appendix restates pass2/04 §3 as the checklist reviewers use against
AJ-E401/AJ-E402. Where this summary and pass2/04 disagree, pass2/04 wins.

- **I-01 Provider.** Icon + abbreviation (`Claude`, `Codex`, `ZCode`).
  Source: `SessionInfo.harness`. Static per Instance. Unknown Provider
  renders a generic marker + text, never blank. Refresh: none.
- **I-02 Account.** Initials disc, per-Account hue, short label. Source:
  `Account.label` via AJ-E401 join. Logged-out renders a hatched avatar
  plus a `Sign in` text chip. Hue is never the only identity channel.
- **I-03 Model.** Running model when observed (AJ-E412), else launch
  model, else `default model`. Drift from launch is marked in text.
- **I-04 State.** Icon + text + colour for the 9 wire states
  (`model.rs:230-238`). Event-driven, target < 300 ms to the row.
- **I-05 Quota.** Bar + `% left` + reset countdown + tier tag
  (`Meas/Comp/Est`). Poll 30 s plus usage events; countdown ticks 1 min.
  No feed renders a dashed bar + `Quota: no data`.
- **I-06 Cost.** Session tokens always; dollars only with known pricing,
  labelled `Est - value-equiv.`. No pricing renders tokens only.
- **I-07 Resources.** 60 s sparkline (solid RAM, outline CPU) + current
  `CPU% - RSS` text. Sample 2 s, repaint 5 s. Unmeasurable trees render
  `resources unavailable`, never a measured-looking zero.
- **I-08 Children.** Child-process and subtask count badges. Hidden at 0.
  After parent exit, surviving children move to the AJ-E410 orphan section.
- **I-09 Git.** `branch - dirty-count` from the Instance `cwd`. Omitted
  entirely outside a git worktree; detached HEAD renders
  `detached@<short-sha>`. Refresh: `.git` watch + 15 s poll.
- **I-10 Attention.** One badge, highest severity only: Failed,
  QuotaBlocked, WaitingUser (with wait duration), Stuck?. Event-driven;
  the waiting timer ticks every 15 s. Snoozed renders `snoozed Nm`.
- **I-11 Age.** `42m - active 10s ago` from `started_at`/`last_activity`
  (`model.rs:409-411`). Tick 15 s. No output yet renders `no output yet`.

Card variant rule (AJ-E402 req 4): the project "On it now" card shows the
same eleven fields with a 5-minute sparkline. A field invented for the
card, or dropped from the card, is a defect in either direction.

## Appendix G — Inspector matrix checklist (6 tabs x 5 selections)

Reviewers initial each cell only when its content task's evidence exists.
`n/a` is not an allowed entry: D-06 requires an honest empty state instead.

| Tab | Instance | Session | Provider | Account | Project |
|---|---|---|---|---|---|
| Overview | AJ-E407 r1 | AJ-E407 r5 | AJ-E408 r1 | AJ-E408 r3 | AJ-E408 r5 |
| Usage | AJ-E407 r2 | AJ-E407 r5 | AJ-E408 r2 | AJ-E408 r4 | AJ-E408 r5 |
| History | AJ-E407 r3 | AJ-E407 r5 | AJ-E408 r2 | AJ-E408 r4 | AJ-E408 r5 |
| Quality/Performance | AJ-E409 | AJ-E407 r5 | AJ-E408 r2 | AJ-E408 r4 | AJ-E408 r5 |
| Processes | AJ-E410 r2 | AJ-E407 r5 | AJ-E408 r2 | AJ-E408 r4 | AJ-E410 r3 |
| Actions | AJ-E407 r4 | AJ-E407 r5 | AJ-E408 r2 | AJ-E408 r4 | AJ-E408 r5 |

Reading the matrix: `AJ-E407 r5` means the Session column is specified by
AJ-E407 requirement 5; the AJ-E410 rows own the orphan contract wherever
it appears (Instance, Project, and rollups read-only above Instance).

Two cells deserve explicit reviewer attention because they are the ones
implementers most often shortcut:

- **Session / Processes** is read-only and historical (final journal tree
  snapshot, AJ-E407 req 5). Giving it live kill controls fails review.
- **Provider / Quality-Performance** mixes confidence tiers across
  Accounts; the series must be labelled `mixed` (AJ-E408 req 2), or the
  aggregate invents precision the inputs do not have (D-07).

## Appendix H — Handover protocol for the next agent

1. Read this file's Appendix A/B first, then only the task you are picking
   up, then the pass2/04 section it cites. Do not re-read all of pass2.
2. Confirm every `Depends:` task merged with its evidence artifact
   (Appendix A, "Definition of unblocked"). If evidence is missing, the
   dependency is not done — finish the evidence, do not code around it.
3. Work on a feature branch; keep `main` green. Run `just verify` before
   claiming any requirement done, and paste its tail into the PR.
4. If reality contradicts a requirement (a Provider exposes less than the
   task assumes), the honest-fields rule decides: ship the field as
   `unavailable` + reason, note the contradiction in the PR, and flag it
   for the task owner. Never fabricate, estimate silently, or delete the
   field to make a screenshot pass.
5. If a decision in `docs/review/decision-log.md` appears wrong, do not
   relitigate it inside E4. Implement the settled decision and raise the
   challenge as a decision-log amendment citing file + section.
6. Update the picked task's heading `Status: TODO` only when its Evidence
   artifact exists at the named location and meets its stated threshold.
7. Leave the working tree clean of scratch fixtures; committed fixtures
   belong beside the tests that consume them (Appendix C).

*End of E4 task file. Source of truth for scope disputes: decision-log
D-06/D-07/D-12 first, then pass2/04 section cited in the task, then SPEC.*

*D-17 unblock mapping (2026-10-13, this task's completion PR)*:
- Decision D-17 recorded (maintainer grant quoted) — the ONE audited
  `unsafe` call lives in `crates/juggler-scap-init` (the only crate not
  inheriting the workspace lint baseline; SAFETY audit in source).
- The original crash had a second cause on CLT-only machines: the empty
  `libswift_Concurrency.dylib` stub shadowed the OS runtime —
  `ensure_swift_runtime.sh` now puts `/usr/lib/swift` FIRST in rpath
  order (shared-cache runtime supplies real `_swift_task_*` symbols).
- `ScreenshotSource` is live (init via the shim; empty-desktop
  enumeration maps to `PermissionDenied` — macOS 15+ TCC filtering);
  `ParkedSource` deleted; UI copy `Live preview is not supported on
  this platform` for the remaining non-macOS phase.
- Evidence: `real-capture-live.log` (LIVE PNG frames, 1.7–2.3% CPU
  polling) + `sample-frame.png` (real 1280×505 capture) in
  `docs/qa/evidence/AJ-E415/`.
