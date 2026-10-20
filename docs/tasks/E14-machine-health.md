# E14 — Machine health and resource monitoring

> Source: the maintainer's own words in
> [expected-worfklow.md](../expected-worfklow.md) ("for each tool, session,
> we should show how much resources it is using, cpu, ram, did it spawn any
> sub tasks, sub processes, and allow to track those", plus zombie/leftover
> cleanup); [PROPOSAL.md](../PROPOSAL.md) G7 and §6.5 "Machine health &
> crash recovery"; §6.2 "Every running Instance across all Providers in one
> live view"; [SPEC.md](../SPEC.md) §5.3 "Resource manager" and the M2
> milestone row. The plan delivered the per-session half (AJ-E410
> collector, AJ-E512 hog alerts, the Cleanup Centre) but never tasked the
> machine half: system signals, footprints, the launch advisor, overload
> alerts and recommendations. Raised by the maintainer while dogfooding
> 0.0.6 (2026-10-20): resources are "a big part of our offering".
>
> **Law for every task here:**
> - **Calm until it matters (D-25).** Numbers appear where they answer a
>   question ("can this Mac take another session?", "which session is
>   heavy?"); colour only for what needs you; every state in words too.
> - **Passive only.** Alerts and advice never stop, pause or launch
>   anything; the only autonomous behaviour is a notification (PROPOSAL
>   §6.4).
> - **Unowned is untouchable (D-08).** Processes AI Juggler did not start
>   are observed read-only and labelled "not started by AI Juggler"; no
>   stop, kill or cleanup control is ever offered on them, in any tier.
> - **Read only what is needed.** Of another process's environment only
>   `CLAUDE_CONFIG_DIR` is read (as `running_setups` already does);
>   environments are never logged, stored or sent.
> - **Honest numbers.** A value the OS does not report is "—" with a
>   reason, never a fabricated zero; projections say what they are based
>   on and how many sessions they learned from.
> - **Safe code.** `unsafe` stays forbidden in our crates; OS facts come
>   from `sysinfo`, `objc2-*` safe bindings or a `sysctl` read.

### AJ-E1401 Machine signals: CPU, memory and pressure, swap, disks in use, power and thermal | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E410
**Goal**: AI Juggler knows how the Mac itself is doing, sampled cheaply on the existing staggered tick, with a short history.
**User story**: As a developer running several AI sessions, I can see whether my Mac is fine, getting tight or struggling — in words, not only numbers.
**Requirements**:
1. A `MachineSampler` in `juggler-supervisor` SHALL sample, on the 2 s processes tick: whole-machine CPU % and core count; memory total, available, and used as Activity Monitor counts it (app + wired + compressed); the kernel's memory-pressure level (macOS `kern.memorystatus_vm_pressure_level`: normal / warning / critical); swap total and used. Disk free/total SHALL be sampled every 30 s for the volumes in use (the data dir's volume and the volumes holding running sessions' folders). Power (on battery or on power, minutes left when the OS estimates them, the OS low-battery warning, Low Power Mode) and thermal state (nominal / fair / serious / critical) SHALL be sampled every 60 s where the OS reports them; elsewhere they are `None` with a reason.
2. A ring SHALL keep 5 minutes of the 2 s samples (150) for CPU, memory used, pressure and swap used, so alerts can tell a spike from a sustained state and swap growth can be measured (MB/min).
3. `Pressure` SHALL be summarised in plain words by one pure function used by every surface: "Plenty of room" / "Getting tight" / "Under pressure", with the reasons that put it there (pressure level, available memory < 10 %, swap growing > 100 MB/min, sustained CPU > 90 % for 60 s, thermal serious/critical, a volume in use < 20 GB free; < 5 GB is pressure).
4. A `machine_health` command SHALL return the latest sample, the summary and the 5-minute series; the mock bridge SHALL serve a deterministic fixture for each summary state.
**UI/UX reasoning**: One vocabulary for the machine everywhere (NN-g #4 consistency); words before gauges (D-25 plain words).
**Performance budget**: the sampler adds ≤ 2 ms p95 to the tick (CPU + memory refresh only; disks/power/thermal on their slower cadences); no new `System` construction per tick (the AJ-E504 construction counter stays flat after warm-up); < 1 % CPU overall for resource monitoring (SPEC §1).
**Quality gates**: unit tests of the summary rules on fixture samples (each reason alone, combinations, missing values); ring capacity test; construction-counter test; a live smoke test that reads real values on the build Mac (non-zero total memory, CPU in 0–100 × cores).
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1401/manifest.md` with the test log and one live sample from the reference Mac.
**Visual testing**: none (AJ-E1404 shows it).
**Files to touch**: `crates/juggler-supervisor/src/machine.rs` (new), `crates/juggler-supervisor/src/lib.rs`, `crates/juggler-supervisor/Cargo.toml`, `crates/juggler-app/src/machine_health.rs` (new), `crates/juggler-app/src/commands.rs` (tick hook), `crates/juggler-app/src/lib.rs` (command), `ui/src/lib/bridge.ts`, generated types.
**Research basis**: PROPOSAL §6.5 "Monitor"; SPEC §5.3; Apple `ProcessInfo.thermalState`, `IOPSGetTimeRemainingEstimate`, `kern.memorystatus_vm_pressure_level`.
**Handover note**: Pressure level is the kernel's own judgement — trust it over "used memory", which on macOS is always high (file cache). Swap *used* stays high for days after a squeeze; swap *growth* is the thrash signal.

### AJ-E1402 See the AI tools AI Juggler didn't start (read-only) | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E1401, `running_setups` (PR #62)
**Goal**: The Claude desktop app's windows and `claude` running in a terminal count in "what your AI tools use", attributed to the Account whose folder they use, without ever being treated as ours.
**User story**: As someone who also opens Claude in its own app and in a terminal, AI Juggler's numbers include those too, and say plainly which ones it didn't start.
**Requirements**:
1. From the `running_setups` classifier, each running Claude desktop app instance and each terminal `claude` SHALL become an *observed* tree: its root pid, place (`desktop` / `terminal`), config folder, and the Account whose state folder (or imported source folder) matches — or "not one of your accounts".
2. Each observed tree's CPU % and memory SHALL be measured over its whole process tree (the desktop app's helpers included) on the 2 s tick, reusing the collector's table (no second full scan); a tree supervised by AI Juggler SHALL never also count as observed (no double count).
3. Observed trees SHALL be labelled "not started by AI Juggler" everywhere they show, and SHALL never be offered stop, kill, cleanup or the Processes tab's stop controls, in any tier (D-08). The ledger is never written for them.
4. The environment read stays `CLAUDE_CONFIG_DIR`-only; nothing about observed processes is logged or stored except the aggregate footprints of AJ-E1406 (by place, never by command line).
5. Decide, with evidence from walking the rail with and without observed rows, whether observed tools appear in the rail (e.g. a quiet "Also running" group) or only in the This Mac panel; record it as a D-25 amendment in the decision log.
**UI/UX reasoning**: Visibility of system status (NN-g #1) without false ownership: the user sees everything that uses their Mac, and the app never pretends it can manage what it didn't start.
**Performance budget**: classification + tree sums ≤ 10 ms p95 per tick at 20 observed trees; the environment scan runs on the full-scan cadence (every 5th tick), not every tick.
**Quality gates**: fixture process tables (desktop app with helpers, terminal `claude` under a shell, a `claude` started by AI Juggler, an unrelated `node`) prove attribution, no double count, and that no stop path accepts an observed pid (a test calls each stop/cleanup entry point with an observed pid and gets a refusal).
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1402/manifest.md`: test log, a live reading on the reference Mac (counts by place, totals), the D-25 amendment.
**Visual testing**: covered in AJ-E1404/E1405 captures.
**Files to touch**: `crates/juggler-app/src/running_setups.rs`, `crates/juggler-app/src/machine_health.rs`, `crates/juggler-supervisor/src/supervisor.rs` (read access to the table), `docs/review/decision-log.md`.
**Research basis**: `running_setups.rs` (0.0.6 dogfood: the default `~/.claude` was the sixth Claude desktop window); D-08; PROPOSAL §4 "Every running Instance across all Providers".
**Handover note**: "Observed" is a read model, not an Instance: it has no ledger row, no session id and no actions. Keep it that way.

### AJ-E1403 The This Mac read model | Status: DONE-EVIDENCED | Priority P0 | Effort S | Depends: AJ-E1401, AJ-E1402
**Goal**: One payload answers "how much are my AI tools using right now, which sessions are heaviest, is anything left behind, how is the Mac".
**User story**: As a user glancing at Home, one panel tells me the machine story.
**Requirements**:
1. `this_mac` SHALL return: totals for AI tools (CPU % of the whole Mac, memory bytes and share of RAM) split started-by-AI-Juggler / not; the top 3 trees by memory (session or observed, each with its place words, CPU, memory, helper count); the leftover-process count and held-port count from the existing inventory; and the machine summary of AJ-E1401.
2. A session is *heavy* (for rail notes and Needs you) when its tree is above 90 % CPU sustained 60 s, above 2 GB, or flagged by an AJ-E512 hog rule; the payload carries the per-session heavy note ("CPU 95%", "3.2 GB").
3. Aggregation SHALL meet the SPEC §5.3 budget: ≤ 200 ms at 50 Instances (measured, in the load tests).
**UI/UX reasoning**: One summary, then depth on request (D-25).
**Performance budget**: ≤ 200 ms at 50 Instances; ≤ 20 ms typical.
**Quality gates**: unit tests for totals, top-3 ordering, heavy rules; a load test at 50 fake trees.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1403/manifest.md` with the test log and the 50-Instance timing.
**Visual testing**: AJ-E1404.
**Files to touch**: `crates/juggler-app/src/machine_health.rs`, `crates/juggler-app/tests/`, `ui/src/lib/bridge.ts`.
**Research basis**: SPEC §5.3 budget; AJ-E512 hog rules.
**Handover note**: Reuse `hog_facts`; never add a second definition of "too much".

### AJ-E1404 Home: the This Mac panel and heavy sessions under Needs you | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E1403
**Goal**: Home answers "is my Mac OK with all this?" at a glance, and a session hogging the machine shows up where things that need you show up.
**User story**: As a user, I see on Home how much my AI tools use, the heaviest sessions, leftovers to clean, and the Mac's state in words; a runaway session appears under Needs you.
**Requirements**:
1. Home SHALL show a "This Mac" panel beside Runway: one line in words ("Plenty of room" / "Getting tight — memory" / "Under pressure — swap is growing"), AI tools' CPU and memory now, the top sessions (a tap opens the session; an observed tool shows "not started by AI Juggler" and opens nothing), and "N leftover processes" linking to Cleanup only when N > 0.
2. Only when the Mac is not "Plenty of room" does the panel use colour; the state is always also a word and a shape (D-25).
3. A heavy session (AJ-E1403 Req.2) and a machine under pressure SHALL appear under Needs you with one sentence and one action ("Using 3.2 GB — Take a look", "Your Mac is under memory pressure — See what's using it").
4. The panel hides itself until the first sample exists; with no AI tools running it reads "Nothing running — your Mac has room".
**UI/UX reasoning**: Lead with "what needs me?"; runway and room both in view; depth one tap away (D-25).
**Performance budget**: the panel refreshes on the 5 s resource emission; no extra timer below 5 s.
**Quality gates**: vitest for each summary state, top-list behaviour, the observed row's lack of actions, Needs you entries; Playwright journey; a11y gate 0 critical/serious; visual baselines.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1404/manifest.md`; hand walk in the built-in browser at 1440×900, 1024×680 and 390×844 in both themes.
**Visual testing**: gallery `72-this-mac/` (plenty, tight, pressure, nothing running, with an observed tool) light and dark.
**Files to touch**: `ui/src/screens/Home.tsx`, `ui/src/components/ThisMacPanel.tsx` (new), `ui/src/lib/bridge.ts`, `ui/tests/visual/`.
**Research basis**: D-25 Home structure; PROPOSAL §6.5.
**Handover note**: Home's three questions stay first; This Mac sits with Runway, never above Needs you.

### AJ-E1405 The rail notes a heavy session; the session header shows CPU · memory · helpers | Status: DONE-EVIDENCED | Priority P0 | Effort S | Depends: AJ-E1403
**Goal**: Resource use is visible where the session is, without cluttering calm rows.
**User story**: As a user scanning the rail, a heavy session tells me so; inside a session I see its CPU, memory and helpers without opening Details.
**Requirements**:
1. A rail row SHALL show a short note only when its session is heavy ("CPU 95%", "3.2 GB"); otherwise nothing changes.
2. The session view's header line SHALL show "CPU 12% · 1.1 GB · 3 helpers" (helpers = child processes) for a live session; the Details line keeps them too.
3. Notes and the header use the same formatter as Home.
**UI/UX reasoning**: Calm rows (D-25 "a row shows quota only when it's low" — the same rule for resources).
**Performance budget**: no new timers.
**Quality gates**: vitest; Playwright; a11y; visual baselines (rail and session panel captures change only where a heavy fixture exists).
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1405/manifest.md`; walked by hand.
**Visual testing**: `72-this-mac/rail-heavy`, `72-this-mac/session-header`.
**Files to touch**: `ui/src/components/WorkflowRail.tsx`, `ui/src/components/RightInspector.tsx`, `ui/src/lib/bridge.ts`.
**Research basis**: D-25.
**Handover note**: The rail is shared with ux-stage4's narrow-rail work; keep the note in one span that folds away when the rail is narrow.

### AJ-E1406 Footprints per launch setting, learned from history | Status: TODO | Priority P0 | Effort M | Depends: AJ-E1401
**Goal**: AI Juggler learns how heavy each kind of session really is on this Mac.
**User story**: As a user, the advice I get is based on what my sessions actually used, not a guess.
**Requirements**:
1. Each supervised session SHALL record its footprint when it ends (and every 10 min while it runs, so a crash still leaves one): peak tree memory, p95 tree CPU, peak helper count, duration; keyed by Provider, Account and launch setting (recipe id or "default").
2. A storage table `footprints` SHALL keep the last 50 per key; queries return median peak memory, median p95 CPU, and the sample count.
3. Observed tools (AJ-E1402) SHALL record footprints keyed by place only (`desktop` / `terminal`), never a command line or folder.
**UI/UX reasoning**: n/a (data).
**Performance budget**: one insert per session end or 10 min; query ≤ 5 ms.
**Quality gates**: storage migration test; median/p95 tests; retention test; crash-path test (a session killed mid-run keeps its last periodic footprint).
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1406/manifest.md`.
**Visual testing**: none.
**Files to touch**: `crates/juggler-core/src/storage/footprints.rs` (new), `crates/juggler-core/src/storage/mod.rs`, `crates/juggler-app/src/machine_health.rs`.
**Research basis**: PROPOSAL §6.5 "per-recipe historical footprints"; SPEC §12 storage.
**Handover note**: Footprints are local only; they never travel in a backup's shareable parts or in usage stats.

### AJ-E1407 The machine launch advisor in New session | Status: TODO | Priority P0 | Effort M | Depends: AJ-E1401, AJ-E1406
**Goal**: Before starting a session, the user knows whether this Mac can take it.
**User story**: As a user about to start another session, the sheet tells me "This Mac can take it", "Tight" or "Not now", and why.
**Requirements**:
1. The New session sheet SHALL show a machine verdict beside the existing quota verdict: OK / Tight / No, from projected available memory after the launch (current available − the setting's median peak, or a conservative default of 1 GB with "not measured yet" until 3 samples exist), memory pressure, sustained CPU and thermal state.
2. Rules: No when pressure is critical or projected available < 5 % of RAM; Tight when pressure is warning, projected available < 15 %, CPU sustained > 85 %, or thermal serious; else OK. Each verdict carries its reasons in words and what would make it safe ("closing <heaviest session> frees about 2.1 GB").
3. The verdict warns; it never blocks or disables Start (AJ-E306 Req.7 rule).
4. Projections SHALL be checked against observed peaks on fixtures: within ±20 % after calibration (SPEC §1).
**UI/UX reasoning**: "Warn before it hurts" (PROPOSAL §6.5); never a gate the user can't pass.
**Performance budget**: the verdict ≤ 20 ms; computed when the sheet opens and when the setting changes.
**Quality gates**: unit tests per rule; fixture tests with the fake Provider spawning memory-hog children (SPEC §12) proving projection accuracy; vitest/Playwright for the sheet; a11y; visual.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1407/manifest.md`; walked by hand.
**Visual testing**: `73-launch-advisor/` ok, tight, no, not-measured.
**Files to touch**: `crates/juggler-app/src/machine_health.rs`, `ui/src/screens/NewSession.tsx`, `ui/src/lib/bridge.ts`.
**Research basis**: PROPOSAL §6.5; SPEC §5.3 admission advisor; AJ-E306's quota verdict (kept separate: quota is the account's room, this is the Mac's).
**Handover note**: Two verdicts, two questions: "Has this account got room?" and "Has this Mac got room?" Never merge them into one colour.

### AJ-E1408 Runtime overload alerts | Status: TODO | Priority P0 | Effort M | Depends: AJ-E1401, AJ-E1403
**Goal**: The user hears about overload while it's happening, before the Mac grinds.
**User story**: As a user deep in work, I get one calm notification when my Mac comes under pressure, with what's using it.
**Requirements**:
1. Alerts SHALL fire for: memory pressure warning/critical sustained 30 s; swap growing > 200 MB/min over 2 minutes; whole-Mac CPU > 90 % sustained 2 minutes; thermal serious/critical; a volume in use under 20 GB free; a session's tree crossing the heavy rules (AJ-E1403).
2. Each alert is a passive notification (the existing notification prefs apply), at most once per kind per 30 minutes, cleared when the condition clears; it names the top contributor in words. No automatic action of any kind.
3. Alerts SHALL also appear under Home's Needs you while active (AJ-E1404 Req.3).
4. Heuristics SHALL pass a fixture corpus driven by the fake Provider spawning memory-hog and CPU-hog children (SPEC §12): every alert fires on its fixture, none fires on the calm fixture.
**UI/UX reasoning**: Calm until it matters; one notification, not a stream.
**Performance budget**: evaluation on the 2 s tick ≤ 1 ms.
**Quality gates**: fixture corpus; rate-limit test; clear-on-recovery test; notification prefs respected.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1408/manifest.md`.
**Visual testing**: `74-overload/` the Needs you entries and the notification text.
**Files to touch**: `crates/juggler-app/src/machine_health.rs`, `crates/juggler-adapters/src/fake.rs` (hog children, if not present), `crates/juggler-app/tests/`, `ui/src/screens/Home.tsx`.
**Research basis**: PROPOSAL §6.5 "runtime alerts"; SPEC §5.3; D-08 (no automatic action).
**Handover note**: The AJ-E512 hog alerts stay the authority for "this session's tree is too heavy"; this task adds the machine-wide ones and feeds both to one place.

### AJ-E1409 Recommendations from real data | Status: TODO | Priority P1 | Effort S | Depends: AJ-E1406
**Goal**: Turn footprints into advice the user can act on.
**User story**: As a user, I learn how many sessions this Mac comfortably runs and which launch setting is heaviest.
**Requirements**:
1. With ≥ 3 footprints, This Mac SHALL offer: "This Mac comfortably runs about N Claude sessions at once" (N from RAM headroom ÷ median peak, keeping 20 % free), and "<setting> is the heaviest launch setting — about X GB a session" when one setting's median is ≥ 1.5× the others'.
2. Each recommendation says what it's based on ("from your last 12 sessions") and hides until the data exists.
**UI/UX reasoning**: Advice from the user's own data (PROPOSAL §6.5 "Advise").
**Performance budget**: computed with This Mac; ≤ 5 ms.
**Quality gates**: unit tests on footprint fixtures.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1409/manifest.md`.
**Visual testing**: `72-this-mac/recommendations`.
**Files to touch**: `crates/juggler-app/src/machine_health.rs`, `ui/src/components/ThisMacPanel.tsx`.
**Research basis**: PROPOSAL §6.5 "Advise".
**Handover note**: Never recommend from fewer than 3 sessions.

### AJ-E1410 Budget proof and the M2 resource-manager gate | Status: TODO | Priority P1 | Effort S | Depends: AJ-E1401..AJ-E1409
**Goal**: Show the resource manager meets its SPEC budgets on the reference Mac.
**User story**: As the maintainer, I can see monitoring costs < 1 % CPU and the advisor's projections land within ±20 %.
**Requirements**:
1. Measure monitoring overhead (AI Juggler's own CPU with and without the machine sampler) over 10 minutes with 5 sessions; ≤ 1 %.
2. Record the This Mac aggregation timing at 50 Instances and the advisor's projection error on the fixture corpus.
**UI/UX reasoning**: n/a.
**Performance budget**: as SPEC §1 / §5.3.
**Quality gates**: the load-budget test file gains the machine-health budgets.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1410/manifest.md`.
**Visual testing**: none.
**Files to touch**: `crates/juggler-app/tests/load_budgets.rs`, evidence.
**Research basis**: SPEC §1, §5.3.
**Handover note**: Measure on a quiet machine; record load average with every number.
