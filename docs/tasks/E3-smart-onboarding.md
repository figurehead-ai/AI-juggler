# E3 — Smart Onboarding & Provider Setup

> Epic owner: onboarding. Settled decisions are not re-litigated: D-02/D-10
> (Providers/Accounts/Instances; Profile = "Launch settings"), D-12 (renewal
> date user-entered, optional, never guessed), D-14 (pack/billing secret
> policy is always the user's remembered choice, global + per-Provider
> override, asked explicitly at first run). Sources: `docs/review/pass2/03-smart-onboarding-and-provider-setup.md`,
> `docs/review/deep/03-ui-onboarding-handover-settings-audit.md`,
> `docs/SPEC.md:409` (§12.1), `docs/plan/EPICS.md:63,120`.
> Rule for every task: scanning is metadata-only (ADR-0006); no credential
> contents are read, copied silently, logged, or telemetried; every
> state-changing action executes only on an explicit user click
> (`docs/SPEC.md:26` automation invariant).

### AJ-E301 Parameterise the wizard by Provider and add the Step-0 install gate | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: none
**Goal**: One wizard shell serves every Provider instead of the Claude-only constant, and a missing CLI is caught before any Account is created.
**User story**: As a new user without `claude` installed, I see "Claude Code isn't installed" with install help at step 0, so I never waste three steps reaching a dead Login.
**Requirements**:
1. The wizard SHALL take a `provider: HarnessId` parameter; `const HARNESS = "claude-code"` SHALL be removed and all call sites (first-run, Profiles Add account, Summary loop-back) SHALL pass the Provider explicitly.
2. Step 0 SHALL call `discover()` per Provider and render one row per Provider: `Installed vX.Y` / `Not installed` / `Installed (vX, unverified)` against `verified_against`.
3. A Provider with no binary SHALL show [Install <Provider> →] (vendor page) and [Continue without it]; selecting it SHALL NOT advance to Identity.
4. Backend errors in the funnel SHALL be coded (`code`, `retryable`, `user_message`) end-to-end; the UI SHALL never render a raw `to_string()` / OS string.
5. The wizard SHALL disable its close control while an import/create call is in flight, or SHALL confirm "Setup is still running — it will appear in Accounts" before closing.
6. Re-entry with zero Accounts after dismissal SHALL re-offer the wizard from Accounts with copy "Finish setting up AI Juggler".
**UI/UX reasoning**: Nielsen H5 (error prevention) + H9 (recover from errors): prevent the dead-end instead of explaining it. States: checking (skeleton rows, ≤1 step), installed, not-installed, unverified. Copy: "Claude Code isn't installed on this Mac." / button "Install Claude Code →" / link "Continue without it". Shortcut: `Esc` closes only when idle; `⌘W` never closes mid-import. Focus moves to the primary button on step change; dialog has `role="dialog"`, `aria-label`, focus trap and return-focus (Deep-03 §8).
**Performance budget**: `discover()` for 3 Providers ≤ 800 ms p95 total (parallel, 2 s hard cap per Provider); wizard first paint ≤ 300 ms; no probe runs on the UI thread.
**Quality gates**: `cargo clippy --workspace -- -D warnings` clean; TS strict clean; no new `window.alert` in `ui/src/` (grep gate); guardrail suite still asserts no credential reads.
**Evidence/validation proof**: `cargo test -p juggler-app --test onboarding` passes incl. new no-binary fixture (fake `PATH` without `claude`) asserting Step 0 renders and Identity is unreachable; Playwright wizard spec screenshots stored under `docs/qa/e3/AJ-E301/`; pass = 100% of the 6 requirements covered by a named test, listed in the PR description.
**Visual testing**: Capture Step 0 at 1280×800 and 390×844 (narrow sheet) in installed / not-installed / unverified states; pixel-diff against approved baseline (≤0.5% differing pixels, human review for copy); a11y: axe-core 0 violations, keyboard-only walkthrough recorded, focus order matches visual order.
**Files to touch**: [OnboardingWizard.tsx](../../ui/src/screens/OnboardingWizard.tsx#L18), [commands.rs](../../crates/juggler-app/src/commands.rs#L347), [error.rs](../../crates/juggler-app/src/error.rs#L52), [bridge.ts](../../ui/src/lib/bridge.ts), [App.tsx](../../ui/src/App.tsx#L94), [Profiles.tsx](../../ui/src/screens/Profiles.tsx#L89), [onboarding.rs](../../crates/juggler-app/tests/onboarding.rs)
**Research basis**: pass2/03 §6(a) Step 0 and §10 slice 1; Deep-03 §3.2 (no harness-missing path), §5 E1/E13; `claude_code.rs:126` `discover()` searched paths.
**Handover note**: Do not fork a second wizard per Provider — parameterise. Trap: account creation succeeds without a binary (it only makes a dir), so the gate must live in the wizard, not the backend create call.

### AJ-E302 Widen the scan engine: aliases, cc-switch, installed CLIs, projects, desktop | Status: DONE-EVIDENCED | Priority P0 | Effort L | Depends: AJ-E301
**Goal**: The scan probes signals S1–S10 (pass2/03 §3) per Provider, in parallel, with per-probe health, instead of four Claude-only origins.
**User story**: As a developer who juggles accounts via shell aliases and cc-switch, I open AI Juggler and every real setup is already listed, including the one I forgot.
**Requirements**:
1. The adapter trait SHALL gain default-implemented `scan_shell_aliases() -> Vec<AliasHint>` and `scan_managers() -> Vec<ManagerHint>`; existing adapters compile unchanged until they opt in.
2. Shell scanning SHALL parse only `~/.zshrc`, `~/.bashrc`, `~/.config/fish/*` for `CLAUDE_CONFIG_DIR=` / `CODEX_HOME=` assignments (path values only, source file + line recorded); it SHALL never evaluate shell code or read other variables.
3. cc-switch scanning SHALL open `~/.cc-switch/cc-switch.db` read-only and return existence + provider-row count + provider names only; on locked/corrupt DB it SHALL degrade to existence-only and never fail the scan.
4. Every probe SHALL have a 2 s budget and return `ScanHealth::{Ok, Partial(reason), Denied}`; the scan result SHALL carry per-probe health to the UI.
5. Probes SHALL run concurrently off the UI thread and stream progress as `n of 10 checks`; total scan SHALL be cancellable.
6. Project seeds (S9) SHALL return directory names only from transcript cwds / `projects/` children; desktop detection (S10) SHALL check `/Applications/*.app` presence only.
7. No probe SHALL read credential file contents or keychain values; keychain checks remain existence-only.
**UI/UX reasoning**: Nielsen H1 (visibility of system status): the unbounded spinner (Deep-03 §1.1) becomes "Checking… 4 of 10" with per-source detail under "Details ▾". Denied state copy: "We couldn't check 2 places — your setups there won't appear." with [Details]; an empty result with any `Denied` probe SHALL never show "No existing setups found". WCAG 2.2 AA: progress is text, not colour-only.
**Performance budget**: Full 10-signal scan ≤ 3 s p95 on a home with 5 config dirs and ≤ 50k files; per-probe ≤ 2 s; scan RSS overhead ≤ 30 MB; `--version` executions ≤ 3 (one per Provider).
**Quality gates**: Fixture-HOME tests for each signal incl. malformed rc file, locked SQLite DB, dangling alias target; static guardrail test asserts no `read_to_string` on credential paths in scan code; clippy pedantic clean.
**Evidence/validation proof**: `cargo test -p juggler-adapters scan` with committed fixture homes under `crates/juggler-adapters/tests/fixtures/homes/`; live-dogfood step: run on the maintainer machine, store the anonymised signal-count table (no paths) in `docs/qa/e3/AJ-E302/scan-dogfood.md`; pass = all 10 signals exercised, 0 credential-content reads (guardrail test + code review checklist).
**Visual testing**: Screenshots of scanning-in-progress (progress text), completed, and partial/denied states at 1280×800; verify no layout shift when progress text updates (CLS check via Playwright trace); a11y: `aria-live="polite"` announces completion once.
**Files to touch**: [environment.rs](../../crates/juggler-adapter/src/environment.rs#L61), [adapter.rs](../../crates/juggler-adapter/src/adapter.rs#L80), [claude_environment.rs](../../crates/juggler-adapters/src/claude_environment.rs#L50), [claude_code.rs](../../crates/juggler-adapters/src/claude_code.rs#L126), [commands.rs](../../crates/juggler-app/src/commands.rs#L336)
**Research basis**: pass2/03 §3 table S1–S10 and scan-contract additions; Deep-03 §1.2 findings 1–2; ADR-0006 Decision 1 (metadata-only).
**Handover note**: Re-verify the live cc-switch DB schema version before reading beyond existence+count (pass2/03 §10 unresolved). Trap: extending the scan must not widen the `.credentials.json` copy exception (`claude_environment.rs:224`) — resolve that per FCUR F4.1 option (b) first or keep the exception Claude-import-only and documented.

### AJ-E303 Found / Likely / Guessed inference with inline confidence display | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E302
**Goal**: Every candidate carries an inferred label, purpose, login state and activity tier, labelled Found/Likely/Guessed, instead of a raw path card.
**User story**: As a first-run user, I see "Claude · Work — signed in" rather than `~/.claude-work`, so I can confirm instead of decoding paths.
**Requirements**:
1. Inference SHALL be deterministic rules (no model call, works offline), evaluated in the pass2/03 §4.1 order, first match wins per field.
2. Each candidate SHALL expose `purpose`, `login_state`, `activity` (Active ≤7 d / Dormant ≤90 d / Archive candidate), `already_imported`, and merged duplicate signals (same canonical dir via alias + home-pattern = one candidate).
3. Confidence SHALL render as the words Found / Likely / Guessed next to each inferred field; numeric scores SHALL NOT be shown.
4. Plan tier SHALL never be inferred from usage volume; with no evidence it stays "Not sure yet", and the plan list SHALL include Max 5×.
5. Every inferred field SHALL be editable inline in one tap/click; editing SHALL NOT re-run the scan.
6. Already-managed candidates SHALL render disabled with "Already in AI Juggler" inline at Detect, never as a post-Identity error.
7. All card notes SHALL render (no `notes[0]` truncation), including the keychain re-login caveat, with Detect copy "signed in here — you'll sign in once more after import" when the probe is keychain-bound.
**UI/UX reasoning**: Nielsen H2 (match the real world) + NN/g confidence-labelling guidance: words beat percentages users cannot calibrate. Card layout follows pass2/03 §4.2 wireframe: line 1 Provider · path · login chip · tier word; line 2 "Likely: Work · Active this week · 3 projects"; actions [Import as "Work"] [Edit] [Ignore]. Ignore is reversible via "Ignored (n) ▾". WCAG: tier conveyed by text + icon, never colour alone; cards are buttons with descriptive `aria-label`s.
**Performance budget**: Inference over ≤ 200 candidates ≤ 50 ms; inline edit commit ≤ 100 ms; no re-scan on edit (asserted by probe call count in tests).
**Quality gates**: Golden-file tests for the rule table (each rule ≥ 1 positive + 1 negative fixture); duplicate-merge test (alias + pattern → 1 candidate); UI test asserts Max 5× present and "Not sure yet" is the no-evidence default.
**Evidence/validation proof**: `cargo test -p juggler-adapters inference` + Playwright card-state screenshots in `docs/qa/e3/AJ-E303/`; dogfood: maintainer confirms/overrides inference on their real scan, override rate recorded per field (target < 30% or the rule is retuned before AJ-E304 ships).
**Visual testing**: Cards in Found/Likely/Guessed × signed-in/not-signed-in/already-managed matrix (9 states) at 1280×800 and 390×844; long-path truncation screenshot (deep t3 path); a11y: axe 0 violations, tier words readable at 200% zoom.
**Files to touch**: [environment.rs](../../crates/juggler-adapter/src/environment.rs#L61), [claude_environment.rs](../../crates/juggler-adapters/src/claude_environment.rs#L83), [OnboardingWizard.tsx](../../ui/src/screens/OnboardingWizard.tsx#L166), [model.rs](../../crates/juggler-core/src/model.rs#L288)
**Research basis**: pass2/03 §4.1 rule table, §4.2 wireframe; Deep-03 §1.3, §2.1 (missing Max 5×, duplicate labels), §2.3 caveat 2.
**Handover note**: Plan honesty is load-bearing — quota tiers (Measured/Computed/Estimated, `model.rs:288`) forbid manufactured precision. Trap: fixing duplicate labels only in the UI leaves pickers ambiguous; collision suffixing belongs to AJ-E304's engine.

### AJ-E304 Recommendation engine: one accept-click juggling set | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E303
**Goal**:
Turn the inferred inventory into one explained set-level proposal
("Claude ×3 + Codex ×1") that the user accepts, customises, or
ignores per item, instead of importing card by card.
**User story**:
As a developer with three Claude dirs and one Codex home, I read
one sentence describing my juggling set and click Accept once, so
first setup takes seconds rather than four separate imports.
**Requirements**:
1. The engine SHALL propose importing every Found + Signed-in
   candidate as its own Account, default ON, each with a one-line
   rationale ("Signed in — import keeps history").
2. Dormant candidates SHALL default OFF under a collapsed
   "Found but quiet" group with one-tap include per item.
3. Each installed-but-unconfigured Provider SHALL be proposed as
   "Add <Provider> (installed, not set up)"; it SHALL NOT be
   created without the accept click.
4. Labels SHALL come from AJ-E303 inference and SHALL be
   collision-suffixed in the engine (`Work`, `Work 2`), never by
   showing a validation error after typing.
5. The most-recent project (S9) SHALL be proposed as
   "Start here: ~/Dev/api" on the proposal and Summary screens.
6. Accept SHALL execute imports sequentially with per-item
   progress; a failed item SHALL degrade that item only and
   SHALL NOT abort the remaining set.
7. The engine SHALL stage only: no Account creation, login, or
   file copy SHALL occur before the accept click (SPEC §26
   automation invariant, `docs/SPEC.md:26`).
8. Every recommendation SHALL be explainable in one line in the
   UI; unexplained recommendations SHALL NOT ship.
**UI/UX reasoning**:
NN/g progressive disclosure + Nielsen H3: default path is one decision ([Use recommended setup →]), Customise expands per-item toggles, Expert… stays visible but out of the path (pass2/03 R2/R3). Proposal copy follows the §5 worked example: "Your juggling set: Claude ×3 (Work — signed in; Personal — signed in; Side — needs sign-in) + Codex ×1 (signed in)." Buttons: [Accept setup →] primary, [Customise], [Expert setup…]. Keyboard: `⌘↩` accepts; `Esc` returns to Detect without discarding edits.
**Performance budget**:
Proposal computation ≤ 100 ms for ≤ 200 candidates; accept of
a 4-Account set shows first per-item progress ≤ 500 ms;
import throughput is bounded by AJ-E307, not this engine.
**Quality gates**:
Deterministic engine unit tests (same inventory → same set,
byte-identical proposal struct); property test: no proposal
ever contains a duplicate label or a managed candidate;
guardrail test asserts zero storage writes before accept.
**Evidence/validation proof**:
`cargo test -p juggler-core recommendation` (or the crate that
hosts the engine) with the pass2/03 §5 worked-example fixture
(3 Claude + 1 Codex) asserting the exact proposal text parts;
Playwright accept-flow run stored in `docs/qa/e3/AJ-E304/`;
pass threshold: partial-failure fixture (item 2 of 4 fails)
ends with 3 imported, 1 failed-with-reason, 0 aborts.
**Visual testing**:
Proposal screen, Customise-expanded screen, and
accept-in-progress screen at 1280×800 and 390×844; states:
all-signed-in, mixed sign-in, zero-found collapse; comparison
by Playwright screenshot diff ≤ 0.5% plus human copy review;
a11y: proposal readable as a single list by screen reader,
axe 0 violations.
**Files to touch**:
[OnboardingWizard.tsx](../../ui/src/screens/OnboardingWizard.tsx#L109),
[model.rs](../../crates/juggler-core/src/model.rs#L145),
[commands.rs](../../crates/juggler-app/src/commands.rs#L392),
[bridge.ts](../../ui/src/lib/bridge.ts)
**Research basis**:
pass2/03 §5 rules 1–6 and worked example; `docs/internal/expected-workflow.md:11`
("making recommendations"); Deep-03 §4 (funnel ends at N=1).
**Handover note**:
Keep the engine pure and UI-free so E4/E5 surfaces can reuse
it. Trap: "import everything found" without the Dormant-off
rule recreates the clutter the maintainer banned
("thousands of options", `expected-workflow.md:11`).

### AJ-E305 Flows (a) first install and (b) new project within click budgets | Status: DONE-EVIDENCED | Priority P0 | Effort L | Depends: AJ-E304
**Goal**:
Ship flow (a) first install at ≤ 5 clicks with 0 required typed
fields, and flow (b) new project at ≤ 3 clicks, both on the
AJ-E301 shell with the AJ-E304 proposal.
**User story**:
As a brand-new user, I go from first launch to a running
Instance in under three minutes; as a daily user, I start a
new project in two clicks from a recent folder.
**Requirements**:
1. Flow (a) SHALL be: Step 0 gate → Scan + proposal →
   Confirm identity (pre-filled) → queued Sign-ins → Summary;
   primary path SHALL be ≤ 5 clicks, 0 typed fields required.
2. The zero-found state SHALL collapse to "Create your first
   account" with a Provider picker (installed first, others
   greyed with install links); the dashed dead-end box SHALL
   be removed.
3. Sign-in steps SHALL queue per Account needing login with
   [Sign in] [Open vendor page] [Copy command] [Skip for now];
   Skip SHALL never block Summary.
4. Summary SHALL offer [Launch first session →] primary,
   [Add another account], and [Done]; Launch SHALL open the
   launch sheet (AJ-E306) with the Start-here project preselected.
5. Flow (b) SHALL start from Projects rail "New project" and
   Summary "Start here": recent dirs (S9) as tiles, native
   [Choose folder…], and drag-drop; typed paths SHALL NOT
   be required.
6. Flow (b) step 2 SHALL preselect the recommended Account
   (last-used on this dir, else most headroom, else most
   recent) with [Change] and [Start session →]; total ≤ 3 clicks.
7. Flow (b) with no Account for the chosen Provider SHALL
   detour inline into flow (d) modally and return to the
   pending project launch on completion.
8. Wizard state SHALL survive an accidental close on the
   Confirm step (label/plan/selection restored on reopen);
   closing SHALL require no confirm when nothing is in flight.
**UI/UX reasoning**:
Apple HIG (forgiveness) + Nielsen H3: every step reversible until its commit click; Back preserves state (Deep-03 §2.1). Step dots show 4 steps for (a); flow (b) is a 2-step sheet. Copy: (a) "You're ready — Work (Claude) is signed in."; (b) tiles show dir name + last-used Account. Shortcuts: `⌘⇧N` New project (D-50, 2026-10-28: `⌘N` is New session, as deep/11 §3 says; it was `⌘N` here until then); tiles arrow-key navigable (WCAG 2.1, visible focus).
**Performance budget**:
Flow (a) scan-to-proposal ≤ 3 s (AJ-E302 budget); click-to-
next-step paint ≤ 200 ms; flow (b) recents list ≤ 100 ms
from cached S9 seeds; time-to-first-Instance p50 < 3 min on
found-set machines (pass2/03 §9 target).
**Quality gates**:
Click-count is a tested contract: Playwright specs assert
the primary-path click counts (a ≤ 5, b ≤ 3) by counting
primary-button activations; funnel events emitted per step
(AJ-E309 schema) with no missing-step gaps in test runs.
**Evidence/validation proof**:
Playwright specs `wizard-first-install.spec` and
`new-project.spec` green in CI, traces + screenshots in
`docs/qa/e3/AJ-E305/`; scripted dry-run on a fixture HOME
(3 Claude + 1 Codex) reaches a launched fake-harness Instance;
pass = both click budgets met and 0 typed fields on path (a).
**Visual testing**:
All 5 steps of (a) + both steps of (b) at 1280×800,
1024×768, 390×844; zero-found and no-Account-detour states;
screenshot diff ≤ 0.5%; a11y: full keyboard completion of
flow (b) recorded, axe 0 violations on every step.
**Files to touch**:
[OnboardingWizard.tsx](../../ui/src/screens/OnboardingWizard.tsx#L49),
[App.tsx](../../ui/src/App.tsx#L87),
[NewSession.tsx](../../ui/src/screens/NewSession.tsx),
[commands.rs](../../crates/juggler-app/src/commands.rs#L280)
**Research basis**:
pass2/03 §6(a)–(b) button tables; Deep-03 §0 funnel map,
§4 (no Launch/Add-another exits), §5 E2/E14.
**Handover note**:
Do not add a project picker to flow (a) beyond the single
Start-here suggestion — the juggling anchor is flow (b)'s
job (ADR-0007). Trap: persisting wizard state must not
persist any credential-adjacent field; labels/plans only.

### AJ-E306 Flows (c) add Provider, (d) add Account, (e) add Instance | Status: DONE-EVIDENCED | Priority P0 | Effort L | Depends: AJ-E301, AJ-E303
**Goal**:
Three distinct daily flows on the parameterised wizard:
(c) add a tool, (d) add a login to a known tool, (e) launch
a run — each within budget (≤4 / ≤3 / ≤2 clicks).
**User story**:
As an existing user, I add Codex as a new Provider in four
clicks, a second Claude Account in three, and launch an
Instance in two, without re-entering setup I already did.
**Requirements**:
1. Flow (c) SHALL list one card per adapter with install state
   and verified-range badge (`verified_against`); Not-installed
   cards SHALL show [Install →] and SHALL NOT be selectable.
2. Flow (c) step 2 SHALL run a Provider-scoped scan and offer
   [Import found] or [Add fresh]; step 3 SHALL reuse the
   AJ-E308 login queue; Summary SHALL offer [Launch] and
   [Add another account on this Provider].
3. Desktop Providers SHALL show a Terminal/Desktop kind badge;
   a single-instance desktop limit SHALL be stated before
   selection, never discovered at launch failure.
   *Amended 2026-10-21 (0.0.11 walk, coordinator's answer to Q4,
   D-25 calm):* the card names the tool and says Installed / Not
   installed; only a desktop tool carries a kind badge (Desktop),
   and the version and verified range move to the tool's own view
   (Req.1's badge lives there, not on the card).
4. Flow (d) SHALL lock the Provider (no harness picker) by
   passing it to the AJ-E301 wizard; its Source step SHALL show
   Provider-scoped scan results plus [Add fresh account].
5. Flow (d) Identity SHALL pre-fill the inferred, collision-
   suffixed label and plan confirm (incl. Max 5×); an already-
   imported source SHALL show "Already imported as «Work»"
   with [Open it] instead of [Import] (needs AJ-E307 provenance).
6. Flow (e) SHALL be a launch sheet, not a wizard: Project
   pre-filled, Account rows showing login state + quota
   headroom with honesty labels, Launch settings (Profile)
   collapsed under "Customise ▸" with Main preselected.
7. Flow (e) SHALL render the admission verdict (OK/Tight/No,
   SPEC §5.2) inline; Tight/No SHALL explain and SHALL never
   hard-block the launch click.
8. Logged-out Account rows in flow (e) SHALL be disabled with
   "Sign in first" and a one-tap detour into the login queue
   that returns to the sheet; unknown headroom SHALL render
   "headroom unknown", never a fabricated percentage.
**UI/UX reasoning**:
Nielsen H2 + D-02/D-10: Provider = tool, Account = login, Instance = run; "profile" never a noun ("Launch settings" only). Flow (e) is an Apple HIG sheet: [Launch →], `⌘↩` launches, `Esc` cancels cleanly. Headroom uses text + bar (WCAG, never colour-only); disabled rows self-explain inline.
**Performance budget**:
Flow (c) card list ≤ 300 ms from cached `discover()` results
(refresh async, stale-while-revalidate); flow (e) sheet open
≤ 200 ms, headroom values from cache ≤ 50 ms, no live quota
fetch on the launch path.
**Quality gates**:
Playwright click-count specs (c ≤ 4, d ≤ 3, e ≤ 2); UI test
asserts the word "Profile" never appears as a rail/sheet noun;
headroom fabrication regression test (unknown → text, no %).
**Evidence/validation proof**:
Playwright traces + screenshots in `docs/qa/e3/AJ-E306/`;
fake-harness e2e: flow (e) launches two Instances on two
Accounts concurrently (EPICS M1-3 acceptance); pass = all
three budgets met on the fixture HOME.
**Visual testing**:
Provider cards (installed / not-installed / unverified /
desktop), launch sheet (OK / Tight / No, logged-out row,
unknown headroom) at 1280×800 and 390×844; diff ≤ 0.5%;
a11y: sheet focus trap + return focus verified, axe 0.
**Files to touch**:
[OnboardingWizard.tsx](../../ui/src/screens/OnboardingWizard.tsx#L18),
[NewSession.tsx](../../ui/src/screens/NewSession.tsx),
[Profiles.tsx](../../ui/src/screens/Profiles.tsx#L89),
[adapter.rs](../../crates/juggler-adapter/src/adapter.rs#L35),
[model.rs](../../crates/juggler-core/src/model.rs#L173)
**Research basis**:
pass2/03 §6(c)–(e) tables, §1 terminology rulings;
Deep-03 §4, §7 (fabricated `percent={0}`); `docs/SPEC.md:237` (§5.2).
**Handover note**:
Flow (e) is a launch, not onboarding — resist adding steps.
Trap: `SessionRecord + InstanceKind` is the Instance record
today (`model.rs:389`); UI copy renames, storage does not
(pass2/03 §1.2 ruling 4).

### AJ-E307 Flow (f) import from competitor/manual dirs, provenance and degraded import | Status: DONE-EVIDENCED | Priority P0 | Effort L | Depends: AJ-E302, AJ-E303
**Goal**:
Import cc-switch / t3 / alias-discovered / manual folders in
≤ 3 clicks, with source provenance, duplicate protection,
per-folder size breakdown, and a settings-only degraded path.
**User story**:
As a cc-switch user with a 3 GB config, I see where the space
is, choose "Import settings only", and finish — instead of a
2 GiB engineer-worded dead end.
**Requirements**:
1. `Account` SHALL persist import provenance (source canonical
   path + origin + imported-at); a migration SHALL backfill
   existing Accounts as provenance-unknown, never guessed.
2. Importing a provenance-matched source SHALL be blocked in
   Detect with "Already imported as «Work»" + [Open it]
   (fixes Deep-03 E4 duplicate-import hole).
3. The Confirm-source step SHALL show source path, origin badge
   (cc-switch / t3 / alias / manual), total size, and per-folder
   breakdown before any copy starts.
4. A source over the 2 GiB ceiling SHALL offer
   [Import settings only — skip history] and [Add fresh];
   the raw ceiling string SHALL never reach the UI.
5. A folder failing the config-evidence check SHALL get a
   plain-language reason + [Pick a different folder]; a source
   deleted between Detect and Import SHALL offer
   "That setup moved — [Rescan]" returning to Detect.
6. Partial failure (disk full mid-copy) SHALL clean up or mark
   the incomplete managed dir and offer [Remove] / [Retry];
   orphan managed dirs SHALL NOT accumulate silently.
7. ~~"Just track it" (adopt-in-place, no copy) SHALL NOT be
   offered unless its own ADR lands; the default and only
   default-path mode is copy-import via `import_account`.~~
   Superseded 2026-10-20 by ADR-0018 (AJ-E1024): using a setup
   where it is is the default, copying is the option.
8. The keychain re-login note SHALL appear at Confirm-source
   time for keychain-bound sources, not first at Login.
**UI/UX reasoning**:
Nielsen H9 + H5: every failure names the cause in user words with exactly one obvious recovery button. Breakdown is a short list ("projects/ — 2.4 GiB"); ceiling explained as "history is large", never "2 GiB import ceiling". [Remove] uses a plain Apple HIG confirm naming the label.
**Performance budget**:
Per-folder size walk ≤ 2 s for ≤ 50k files (bounded walk,
cached from scan); provenance lookup ≤ 10 ms; import copy
streams progress at ≥ 4 updates/s for copies > 1 s; cancel
takes effect ≤ 500 ms.
**Quality gates**:
Source-immutability test (tree hash/mtime snapshot before/
after, extending `onboarding.rs:61`); duplicate-import test;
oversize-fixture test asserting degraded import succeeds and
contains settings but zero transcript files; rollback test
with a filled-disk fake filesystem (or injected copy error).
**Evidence/validation proof**:
`cargo test -p juggler-app --test onboarding` and
`--test import_t3_profiles` green; fixture corpus under
`crates/juggler-app/tests/fixtures/`; live dogfood: import
the maintainer's real cc-switch set, store size/provenance
table (paths redacted to basenames) in `docs/qa/e3/AJ-E307/`.
**Visual testing**:
Confirm-source (normal / oversize / non-config / vanished),
degraded-import confirm, and duplicate-blocked states at
1280×800 and 390×844; diff ≤ 0.5%; a11y: breakdown list is
real list markup, error states use `role="alert"`, axe 0.
**Files to touch**:
[model.rs](../../crates/juggler-core/src/model.rs#L145),
[claude_environment.rs](../../crates/juggler-adapters/src/claude_environment.rs#L148),
[adapter.rs](../../crates/juggler-adapter/src/adapter.rs#L94),
[commands.rs](../../crates/juggler-app/src/commands.rs#L392),
[onboarding.rs](../../crates/juggler-app/tests/onboarding.rs#L61),
[import_t3_profiles.rs](../../crates/juggler-app/tests/import_t3_profiles.rs)
**Research basis**:
pass2/03 §6(f), §7 N3/N8/N10; Deep-03 §2.2 (provenance gap,
orphan dirs), §2.3, §5 E3–E8; `docs/SPEC.md:400` (managed-dir invariant).
**Handover note**:
Provenance is the load-bearing field — AJ-E306(d) and the
Detect badge both depend on it, so land the migration first.
Trap: marker-first writes mean a failed import still looks
managed; cleanup must key off the import session id, not the
marker alone.

### AJ-E308 Login-state taxonomy, auto-detected sign-in queue, and isolation verification | Status: DONE-EVIDENCED | Priority P0 | Effort L | Depends: AJ-E301
**Goal**:
Replace the binary signed-in chip with a login taxonomy
(signed-in / partial / expired / SSO-pending / API-key), auto-
detect completion, and prove per-Account isolation on Summary.
**User story**:
As a user whose OAuth was abandoned halfway, I see "Sign-in
didn't finish" with [Reopen sign-in], and signing out one
Account never flips another Account's chip.
**Requirements**:
1. Login state SHALL be one of SignedIn, Partial, Expired,
   SsoPending, ApiKey, LoggedOut, Unknown (probe failed);
   probe failure SHALL never render as LoggedOut.
2. The wizard SHALL auto re-probe on login-PTY exit and poll
   every 2 s while the Sign-in step is visible; manual
   [Re-check] SHALL remain as a fallback, not the only path.
3. All three login doors (wizard, ReadyHome, Profiles modal)
   SHALL share one queue component with identical states,
   copy, and error handling; no door SHALL use `window.alert`
   or an unhandled promise rejection.
4. Copy SHALL frame the vendor flow: "This is Claude's own
   sign-in. AI Juggler never sees your credentials." plus
   numbered steps (Sign in → follow vendor prompts → return;
   we detect it). SSO state SHALL add "Finish in your
   browser/IdP window, then return here."
5. API-key Accounts SHALL badge "API key account" and show
   quota as Estimated/none, never subscription rings; two
   dirs probing as the same login SHALL warn non-blockingly:
   "These may be the same login twice — juggling needs two
   subscriptions" (pass2/03 N6).
6. Offline during login SHALL persist Accounts, mark Summary
   "sign in when online", and SHALL NOT force a re-scan.
7. After every import/create, the isolation checklist SHALL
   run automatically: managed dir 0700 + marker; launch env
   points at the managed dir only; source tree hash/mtime
   unchanged (import only); per-dir probe independence;
   zero writes outside managed dirs. Any fail SHALL block
   [Launch] (not [Done]) with the failing line named.
**UI/UX reasoning**:
Deep-03 §3.3 wireframe + Nielsen H1: a live status line
("○ Waiting… / ● Signed in ✓ auto-detected") replaces the
hidden Re-check hunt. ReadyHome skip SHALL be persistent
per Account set, not session-only, and SHALL list only
Accounts needing action (Deep-03 §3.2). WCAG: status line
uses `aria-live="polite"`; polling never steals focus.
**Performance budget**:
Probe ≤ 300 ms per Account; poll adds ≤ 1 probe/2 s/Account
and stops when the step unmounts; isolation checklist
≤ 2 s for a ≤ 2 GiB import (hash of metadata, not contents,
except fixture mode); two-Instance concurrency check is a
test-harness step, not a user-blocking wait.
**Quality gates**:
State-machine tests for all 7 states incl. PTY-exit-without-
login (Partial) and stale-credential (Expired) fixtures;
guardrail: credential contents never logged (log-scan test);
concurrent two-Account fixture proves transcript isolation.
**Evidence/validation proof**:
`cargo test -p juggler-app --test onboarding login` plus
Playwright login-queue spec; live dogfood with two real
Claude Accounts (sign out B, assert A unchanged) recorded
in `docs/qa/e3/AJ-E308/`; pass = 7/7 states reachable by
fixture and isolation checklist 6/6 green (EPICS M1 gate,
`docs/plan/EPICS.md:63`).
**Visual testing**:
Sign-in step in Waiting / Partial / Expired / SSO / ApiKey /
SignedIn states, Summary with checklist pass + one forced
fail, at 1280×800 and 390×844; diff ≤ 0.5%; a11y: axe 0,
status announcements verified with screen-reader log.
**Files to touch**:
[OnboardingWizard.tsx](../../ui/src/screens/OnboardingWizard.tsx#L270),
[ReadyHome.tsx](../../ui/src/screens/ReadyHome.tsx#L11),
[Profiles.tsx](../../ui/src/screens/Profiles.tsx#L338),
[terminal.rs](../../crates/juggler-app/src/terminal.rs#L56),
[claude_code.rs](../../crates/juggler-adapters/src/claude_code.rs#L332),
[environment.rs](../../crates/juggler-adapter/src/environment.rs#L21)
**Research basis**:
pass2/03 §7 N1–N12, §8 checklist; Deep-03 §3.1–§3.3, §5
E9–E11; `docs/SPEC.md:130` (env isolation table).
**Handover note**:
Extend `LoginProbe` compatibly — `commands.rs` and the rail
already consume LoggedIn/LoggedOut. Trap: the Detect chip
describes the source's login, the Summary must describe the
copy's; keychain installs differ (Deep-03 §2.3).

### AJ-E309 First-run policy choice (D-14/D-12 hook), expert hatches, and onboarding metrics | Status: DONE-EVIDENCED | Priority P1 | Effort M | Depends: AJ-E305, AJ-E308
**Goal**:
First run explicitly asks the two settled policy choices —
pack secret handling (D-14) and optional renewal dates (D-12)
— and instruments the funnel so inference rules can be tuned.
**User story**:
As a privacy-conscious user, I choose once how secrets in my
context packs are handled, see that choice remembered per
Provider, and optionally add a renewal date so expiry warns me.
**Requirements**:
1. Flow (a) SHALL include one Policy step (counts inside the
   AJ-E305 ≤5-click budget by merging with Confirm) asking
   the global pack secret policy with plain-language options
   (e.g. Strip secrets / Keep with warning / Keep silently —
   final copy from the pack epic); no option SHALL be
   pre-selected as a silent default: the user SHALL choose
   or explicitly pick "Decide later", which SHALL stage no
   pack until chosen.
2. The choice SHALL persist globally with per-Provider
   override storage (schema only in this epic; the switch
   flow consumes it later) and SHALL be changeable in
   Provider settings; onboarding SHALL write the global
   value only on an explicit click.
3. Each Account's Confirm row SHALL offer an optional
   renewal-date field ("Renewal date (optional) — we'll warn
   you before it"); empty SHALL persist as unknown and render
   "expiry unknown", never inferred (D-12).
4. The mandatory D-12 research note SHALL be filed at
   `docs/research/billing-renewal-sources.md` (per-Provider:
   sanctioned API/SDK evidence or "no source found"); until
   a source is verified live, no code SHALL auto-populate
   renewal dates.
5. Every flow SHALL expose [Expert setup…]: raw state-dir
   path (read-only + [Reveal in Finder]), env overlay editor,
   Launch-settings (Profile) CRUD, [Import from folder…],
   and a copyable shell snippet reproducing the setup.
6. Events SHALL be emitted locally (telemetry opt-in only):
   `scan_started/completed{providers,found,health}`,
   `proposal_shown/accepted/customised`,
   `import_started/completed/failed{reason_code}`,
   `login_queued/completed/skipped/expired`, `expert_opened`,
   `first_launch{seconds_since_install}`; events SHALL carry
   no paths, labels, or credential-adjacent data.
7. Inference-override rate SHALL be recorded per field
   (AJ-E303 edits) so rules with > 30% override are flagged
   for retuning; funnel reporting SHALL be a local export,
   not a network call, in this epic.
**UI/UX reasoning**:
D-14 is a consent surface, so Nielsen H3 + WCAG consent
clarity: each option states its consequence in one sentence,
the active choice is restated on Summary ("Packs: strip
secrets (all Providers)"), and "Decide later" is a real,
non-shamed choice. Renewal date uses a native date input
with "Skip" affordance (Apple HIG: optional means skippable
without penalty). Shortcut: none on the Policy step —
deliberate choices get no `⌘↩` fast-path.
**Performance budget**:
Policy step adds ≤ 1 s to flow (a); settings write ≤ 50 ms;
event emission ≤ 1 ms/event off the UI thread; local event
store capped at 10 MB with rotation.
**Quality gates**:
Persistence round-trip tests (global + per-Provider override
+ Decide-later = no pack staging); copy lint asserting no
pre-selected radio in the Policy step markup; event-schema
test asserting forbidden keys (path/label/token) never appear.
**Evidence/validation proof**:
`cargo test -p juggler-core policy` + Playwright first-run
spec choosing each option and asserting Summary restatement;
research note reviewed against live Provider docs with URLs
and access dates; funnel dry-run export stored in
`docs/qa/e3/AJ-E309/`; pass = fresh install → two Accounts
in parallel < 5 min (PROPOSAL success metric) with policy
step included.
**Visual testing**:
Policy step (unchosen / chosen / Decide-later), Confirm row
with and without renewal date, Expert panel open, Summary
restatement — at 1280×800 and 390×844; diff ≤ 0.5%; a11y:
radio group labelled, date input labelled, axe 0 violations.
**Files to touch**:
[OnboardingWizard.tsx](../../ui/src/screens/OnboardingWizard.tsx#L233),
[model.rs](../../crates/juggler-core/src/model.rs#L145),
[Profiles.tsx](../../ui/src/screens/Profiles.tsx#L89),
[commands.rs](../../crates/juggler-app/src/commands.rs#L410),
[bridge.ts](../../ui/src/lib/bridge.ts)
**Research basis**:
`docs/review/decision-log.md` D-14 (first-run must ask,
remembered, per-Provider override), D-12 (user-entered,
optional, research-gated); pass2/03 §6.7, §9 metrics.
**Handover note**:
This task owns the hook and storage, not pack behaviour —
the switching epic consumes the policy and must display the
destination Provider's active policy before executing (D-14).
Trap: never backfill a policy default for existing users;
absence of a choice means "ask before first pack", not strip.
**Task-text fix (2026-10-18, AJ-E309 implementer)**: the policy
STORAGE already exists — AJ-E208 shipped the remembered global +
per-Provider override (`juggler_handover::redaction::RedactionPolicy`,
app settings, `redaction_policy_*` commands) and the switch flow
consumes it. A second store would split the source of truth, so this
task writes E208's store from the first run and the option words are
E208's (Strip / Keep / Ask each time — "final copy from the pack
epic") plus "Decide later" (writes nothing). The persistence evidence
is therefore `cargo test -p juggler-app --test redaction_policy` +
`--test onboarding_policy_metrics` (no `juggler-core policy` module
exists). "Provider settings" is Accounts → Pack secrets (global choice
+ one override row per Provider in use).

**Status report (2026-10-16, AJ-E301)**: DONE-EVIDENCED — evidence
`docs/qa/evidence/AJ-E301/manifest.md`. One wizard shell for every
Provider (`provider` prop; the HARNESS constant is gone; both call
sites pass it explicitly). Step 0 gates on the adapters' own
`discover()`, probed in parallel off the UI thread: Installed vX ·
Not installed · Installed (vX, unverified) per Provider; a missing
binary shows Install→ (vendor page) + Continue without it and the
Continue button never reaches Identity (account creation only makes
a dir — the gate lives in the wizard, per the trap note). Errors are
coded end-to-end (`{code, retryable, user_message}` on EVERY command
rejection; the UI renders user_message + a retry affordance, never a
raw OS string — all 13 render sites converted). Close is the confirm
variant while a create/import is in flight ("it will appear in
Accounts"); Esc closes only when idle; role=dialog + focus trap +
return-focus + primary-button focus on step change. Re-entry with
zero accounts after dismissal re-offers "Finish setting up AI
Juggler" from Accounts. Gallery 34-wizard (14 captures) + a11y 2/2;
the 16-onboarding-detect gallery now walks the gate. onboarding 4/4,
vitest 254/254, playwright 419/419, verify green.

**Status report (2026-10-17, AJ-E302)**: DONE-EVIDENCED — evidence
`docs/qa/evidence/AJ-E302/manifest.md`. The scan now probes S1–S10
(pass2/03 §3): the adapter trait carries default `scan_shell_aliases`
+ `scan_managers` (Claude opts in; others compile unchanged), the rc
parse is name-scoped to CLAUDE_CONFIG_DIR/CODEX_HOME across
zsh/bash/fish (POSIX + `set -x` forms, file+line recorded, malformed
lines ignored, never evaluated), cc-switch opens its DB read-only
immutable with an explicit id+name SELECT (locked/corrupt degrades to
existence-only — pinned per state), S9 seeds are directory names
only, S10 is app presence only. The engine runs probes on their own
threads off the UI thread, streams `scan-progress {done,total}`
events ("Checking… n of 10", aria-live), and `scan_cancel` stops
between groups with the report carrying `cancelled` (an
empty-but-denied/cancelled result NEVER reads "No existing setups
found"). A committed fixture home + per-state cc-switch fixtures
drive 7/7 scan tests; a static guardrail test pins zero
credential-content reads; dogfood counts at
`docs/qa/e3/AJ-E302/scan-dogfood.md` (~1.2 s wall). scan 7/7,
onboarding 5/5, vitest 256/256, playwright 425/425, verify green.

**Status report (2026-10-17, AJ-E303)**: DONE-EVIDENCED — evidence
`docs/qa/evidence/AJ-E303/manifest.md`. Every candidate now carries
deterministic, offline inference (§4.1 rules, first match wins):
purpose (manager name verbatim → work/corp/acme → personal/home/side
→ humanised basename), login (Signed in / May be expired — the empty
credentials file / Not signed in, plus the keychain-bound flag that
renders the "sign in once more after import" caveat), activity (≤7 d
Active, ≤90 d Dormant, else Archive), already-imported (the managed
marker renders a DISABLED "Already in AI Juggler" card at Detect),
and merged duplicate signals (alias + home-pattern → ONE candidate,
signals listed). Confidence renders as the words Found/Likely/Guessed
next to each field — never a score; plan tier has NO inference field
at all (pinned by test) and the picker gains Max 5×. Inline edit
(purpose + plan) never re-runs the scan (pinned by a call-count
spy); Ignore is reversible via the Ignored (n) drawer. Golden
rule-table tests (9/9, each rule positive+negative + the 200-in-50ms
budget), duplicate-merge test, wizard vitest 11/11, gallery
35-candidates (9-state matrix + deep t3 truncation), a11y 2/2 after
three real contrast fixes (incl. the light-theme success token).
Dogfood on this machine: 0 overrides across all fields
(docs/qa/e3/AJ-E303/inference-dogfood.md). ts-rs bindings regenerate
in CI via a committed test.


**Status report (2026-10-17, AJ-E304)**: DONE-EVIDENCED — evidence
`docs/qa/evidence/AJ-E304/manifest.md`. A pure engine
(`juggler_core::recommendation::recommend(candidates, gaps,
start_here)`, UI-free for E4/E5 reuse) turns the inferred inventory
into one set-level proposal: active candidates default ON with a
one-line rationale ("Signed in — import keeps history" / "Active —
you'll sign in after import"), dormant candidates default OFF under
the "Found but quiet" group (one-tap include), and installed-but-
unconfigured providers ride in as "Add Codex (installed, not set
up)" adds — never created before the accept click. Label collisions
suffix in the ENGINE (Work, Work 2, Work 3 — no post-typing
validation error anywhere); the summary sentence reproduces the §5
worked example verbatim; "Start here" (the S9 seed) shows on BOTH
the proposal and the set Summary. The wizard's accept executes
sequentially with a per-item aria-live progress list; a failed item
degrades ITSELF only (its coded `user_message` renders inline) and
the run never aborts — the partial-failure fixture (item 2 of 4
rejects) ends 3 done / 1 failed-with-reason / 0 aborts in both
vitest and Playwright. Staging is pinned twice: the app guardrail
test asserts storage stays EMPTY after proposing, and vitest asserts
zero import calls before a click (SPEC §26). ⌘↩ accepts; Customise
expands/collapses without discarding edits. Engine 6/6 (worked
example, dormant-off, collisions, 64-round property,
byte-identical determinism, 202 items inside the 100 ms budget);
onboarding 8/8; vitest 263/263 (wizard 14/14); Playwright 450/450
(gallery 36-recommend: teaser / customise with the quiet group /
frozen accept-in-progress / partial-failure set-summary = 16
captures; a11y 2/2 after one real contrast fix — the ⌘↩ hint
faint→muted; the light-theme first-scan contrast flash is the
recorded AJ-E806 load-race class, confirmed clean by the re-scan
protocol). Zero-found collapse is AJ-E305's state (its Req.2).
**Status report (2026-10-17, AJ-E305)**: DONE-EVIDENCED — evidence
`docs/qa/evidence/AJ-E305/manifest.md`. Flow (a) is now the tested
contract `wizard-first-install.spec.ts`: gate → scan+proposal →
accept (the pre-filled identity confirmation) → the NEW queued
sign-ins step → Summary — walked end to end in exactly 4
PRIMARY-button clicks (≤ 5) with a page-wide input-event recorder
proving 0 typed fields. The sign-ins step queues one card per
Account needing the vendor login ([Sign in] [Open vendor page]
[Copy command] [Skip for now]); Skip advances and never blocks the
Summary; the accept collects the queue from import/create login
states. The zero-found state collapses to "Create your first
account" with a Provider picker (installed first, others gated with
install links — the dashed dead-end is gone); choosing one lands on
Confirm with the label pre-filled. The Summary carries [Launch
first session →] (opens the launch sheet with Start-here
preselected — wired in BOTH wizard instances), [Add another
account] (fresh remount), [Done]. Flow (b) is the new
NewProjectSheet: recency tiles + native folder picker + drop, then
the PURE preselect rule (last-used on dir → most headroom → most
recent) with [Change] and [Start session →] — completed in 2
clicks in `new-project.spec.ts` (≤ 3), summonable by ⌘N (⌘⇧N since D-50) and the
Projects rail's primary button; no Account detours INLINE into the
add-account flow keeping the pending directory. Wizard state
(label/plan/choices/edits) survives an accidental close on Confirm
via sessionStorage (asserted free of path/token-shaped fields);
idle close still needs no confirmation. A funnel seam
(`aij:funnel` CustomEvents, string keys only) records every step of
both flows with no gaps — AJ-E309's event store subscribes to it.
Dots show the 4 phases (Gate/Scan/Sign-ins/Ready) in a role=group;
tiles are arrow-key navigable (the a11y test completes flow (b)
entirely by keyboard). vitest 272/272 (wizard 17/17, new
new-project-sheet 6/6); Playwright 483/483 (gallery 37-flows = 54
captures: the 5 (a)-screens + zero-found + both (b)-steps + the
detour × 3 viewports × light+dark; +5 flow-gate tests) after two
real a11y fixes (the dots' aria-label needed role=group; the
sign-in command block text-live→text-success for AA on light) and
one E806-class theme-boot race converted to the sanctioned
confirm-rescan verdict (recorded). The time-to-first-Instance p50
metric is AJ-E309's dogfood funnel output — this seam is its
instrument (recorded PARTIAL-BY-DESIGN).
**Status report (2026-10-17, AJ-E306)**: DONE-EVIDENCED — evidence
`docs/qa/evidence/AJ-E306/manifest.md`. Flow (c) is the new
AddProviderSheet: one card per adapter with install state, the
verified-range badge (the wire now carries `verified_against` off the
adapter trait), a Terminal/Desktop kind badge, and the
single-instance desktop limit stated BEFORE selection;
not-installed cards link [Install →] and are never selectable.
Choosing a card runs the wizard LOCKED to that Provider (Req.2: its
detect scan IS the Provider-scoped step — the proposal is [Import
found], the fresh-account link is [Add fresh]; the AJ-E305 sign-ins
queue and Summary exits run unchanged). Flow (d) is the same lock:
Accounts → [Add account] mounts at detect with NO Step-0 gate and
NO zero-found re-picker — a single [Create your first Claude Code
account] with the label PRE-FILLED; 3 primary clicks end to end
(already-managed sources render the disabled "Already in AI
Juggler" card — [Open it] lands with AJ-E307's provenance). Flow
(e) reshapes NewSessionModal into the launch sheet: the recommended
Launch settings ride preselected (the pure pickRecipe rule) with
the SPEC §5.2 admission verdict inline as WORDS — the verdict is
newly wired end-to-end (`WindowDisplay.headroom_tokens` →
`RankedRowInfo.admission` via core's pure `admission_verdict`,
Rust-pinned ok/None/no) — Tight/No explain and NEVER block (Launch
stays enabled, gallery-asserted); logged-out rows carry [Sign in
first] (one tap spawns the vendor terminal, the sheet STAYS);
unknown headroom renders "headroom unknown", never a % (regression
test); Launch settings collapse under [Customise ▸]; ⌘↩ launches;
Esc cancels; a focus trap with return-focus was added and pinned.
Click-count contracts: c=4 ≤ 4, d=3 ≤ 3, e=2 ≤ 2
(flows-cde.spec.ts, counted numerically + the funnel seam); the
Profile-noun law asserted (/\bProfile\b/ absent, "Launch settings"
present). vitest 281/281 (+9 across two new files); Playwright
502/502 (gallery 38-flows-cde = 24 captures; a11y +4); ranking 5/5;
verify green; clippy 0. PARTIAL-BY-DESIGN (recorded): the
concurrent two-Instance fake-harness e2e rides the real-backend
dogfood windows (E7/E10); every UI precondition is pinned here.
**Status report (2026-10-17, AJ-E307)**: DONE-EVIDENCED — evidence
`docs/qa/evidence/AJ-E307/manifest.md`. Accounts now carry import
provenance (canonical source path + origin + imported-at; V21
migration, existing rows stay NULL = provenance-unknown, never
guessed) — written at import time, queryable on the wire, and the
duplicate-import hole is CLOSED: a provenance-matched source rejects
with coded `already_imported` + "Already imported as «Work»" and the
wizard renders the block with [Open it]. The Confirm-source panel
(new `source_inspect` command over a metadata-only bounded walk)
shows path, origin badge, file count, total, and the per-folder
breakdown as real list markup BEFORE any copy — plus the keychain
re-login note (Req.8) and two failure states as role=alert: non-config
→ plain reason + [Pick a different folder]; vanished at import →
"That setup moved" + [Rescan]. An over-ceiling source offers exactly
[Import settings only — skip history] / [Add fresh] — the settings-only
import skips projects//todos/ with sized skip notes and flags the
result degraded; the raw ceiling string never reaches the UI ("The
history in this folder is large"). Mid-copy failures remove THIS
session's partial managed dir before returning (session-scoped by
construction — the marker-first trap closed); orphans never
accumulate. The manual-folder entry on Detect uses the native picker
(0 typed paths; label prefilled "Main"); adopt-in-place exists
nowhere (Req.7). All four quality-gate tests green: immutability
snapshot across full+degraded+inspect, duplicate block, oversize
→ degraded with ZERO transcript files, injected-failure rollback.
adapters environment 12/12 (82 total), onboarding 10/10, vitest
285/285 (wizard 21/21), Playwright 520/520 (gallery 39-import = 16
captures; a11y +2 after three REAL pre-existing identity-step
contrast fixes text-faint→text-muted), verify green, clippy 0.
PARTIAL-BY-DESIGN (recorded): per-number perf instrumentation is
bounded-by-construction; the live maintainer cc-switch dogfood table
lands with the maintainer driving (live-user-data constraint).
**Status report (2026-10-18, AJ-E309)**: DONE-EVIDENCED — evidence
`docs/qa/evidence/AJ-E309/manifest.md`. The first run asks the pack
secret policy (D-14) on the Confirm it already has — the set accept
and the single-account Confirm — with AJ-E208's words (Strip / Keep /
Ask each time) plus a real "Decide later"; nothing is pre-selected
(copy lint), Accept waits with a stated reason, ⌘↩ never answers it,
and the choice writes E208's store ONCE on the accept click (Decide
later writes nothing; no backfill — pinned). Flow (a) = 4 primary +
1 policy click ≤ 5, 0 typed; locked flows (c)/(d) never ask and keep
their budgets. Summary restates the active choice ("Packs: strip
secrets (all Providers)") and each Account's renewal ("renews …" /
"expiry unknown"). Every Confirm row offers an optional native-date
renewal with Skip (D-12, user-entered only); the D-12 note is filed
with per-Provider URLs + access dates ("no source found" ×4).
[Expert setup…] is on the wizard, launch sheet, New project sheet and
every Accounts card: backend-resolved state folder + Reveal, a quoted
shell line from new adapter seams (the hard-coded claude login line
is gone), env overrides per Launch settings (isolation and
credential-shaped keys refused; isolation applied last at launch),
Manage Launch settings, Import from folder, and the local metrics
export. Funnel events go to an allow-list-schema local store (2×5 MB),
off the UI thread; first_launch is computed once by the backend;
override rate = edits ÷ shown, > 30% flagged; dry-run export stored
and regression-checked. Accounts → Pack secrets hosts the global +
per-Provider override. Course corrections to the inherited WIP (a
duplicate policy store, a Reveal button that opened the vendor login
page) and in-passing fixes (the AJ-E806 light-theme contrast flash
root cause, "Profiles"/"profile" copy, the lint's case gap) are in
the manifest. core funnel 6/6, onboarding_policy_metrics 5/5, vitest
311/311, Playwright 596/596 (gallery 41-policy = 32, a11y +4).
PARTIAL-BY-DESIGN (recorded): the < 5 min fresh-install wall-clock
and per-number perf timings ride the maintainer dogfood window.
