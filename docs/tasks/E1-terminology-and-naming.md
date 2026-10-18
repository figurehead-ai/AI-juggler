# E1 — Terminology and Naming: Providers / Accounts / Instances

> Epic owner: Product + Core. Status: TODO (all tasks). Execution order is
> dependency order; do not parallelise across a `Depends:` edge.
> Settled law — do not re-litigate: D-02 (full rename, phased; UI → domain
> aliases → docs; never big-bang) and D-10 (Profile demoted to "Launch
> settings") in [decision-log.md](../review/decision-log.md#L21).
> Sources: pass2/01 §4 (rename map + cost), pass2/08 §3.4 (ADR-0008 +
> Phase A/B/C), pass2/00 §3, ADR-0007 (Instance = occurrence, Session =
> conversation inside it). Stage gate: this epic is implementation; D-15
> Stage-1 docs acceptance does not authorise skipping the ADR task.
> Non-negotiable invariants for every task below:
> (1) Stored wire values never change (`claude-code`, `codex`, `zcode`,
> `terminal`/`desktop`, status strings); only Rust/TS identifiers, column
> names, and user-visible copy change. (2) Old databases and old JSON
> fixtures SHALL open and round-trip byte-identically in values.
> (3) `vendor` stays in compliance/legal sentences (SPEC §8, PROPOSAL
> compliance); it is never find-replaced to Provider. (4) External tools'
> `profile = account` (cc-switch/Codex) is never imported.

### AJ-E101 ADR-0008 terminology charter and rename freeze | Status: DONE-EVIDENCED (zcode-primary, 2026-10-13) | Priority P0 | Effort S | Depends: —
**Goal**: Ratify one canonical glossary (Provider/Account/Instance/Session/Launch settings) so every later rename task cites a single source instead of re-deciding.
**User story**: As the maintainer (P4 Operator), I want the rename rules written once so that contributors stop inventing a fourth synonym per screen.
**Requirements**:
1. The ADR SHALL follow [0000-template.md](../adr/0000-template.md#L1) (Status/Date/Deciders/Context/Decision/Alternatives/Consequences) and be numbered 0008, next after ADR-0007.
2. The ADR SHALL define: Provider = HarnessId (binary named only in tooltip), Account = Account (unchanged), Instance = SessionRecord + InstanceKind, Session = conversation inside an Instance only, Profile → "Launch settings" (UI) / LaunchRecipe (internal), and vendor retained for compliance.
3. The ADR SHALL contain the Phase A/B/C table from pass2/08 §3.4 (UI copy → serde-compatible domain aliases → DB v5 + docs) and state "never big-bang" and "wire values unchanged" as decisions.
4. The ADR SHALL list all three conflicts verbatim: code `Profile` vs maintainer "profiles are complicated", deep-01 B-08 "delete Instances" vs ADR-0007 + mandate (maintainer wins), D10 §23 quota-wedge naming superseded.
5. The ADR SHALL freeze new usage: no new `harness`/`profile`/`session`-as-occurrence identifiers in user copy after merge; adapter-internal `harness` is allowed only behind the alias bridge.
**UI/UX reasoning**: Nielsen NN-g #4 Consistency and standards — one word per concept across rail, modals, and errors; glossary box uses plain language per D-09 ("zombie" first-use rule as precedent). No UI ships in this task; the glossary wording itself SHALL be ≤160 characters per definition for tooltip reuse.
**Performance budget**: Docs only; ADR file ≤250 lines, `wc -l` checked in review; zero runtime impact (0 ms, 0 MB).
**Quality gates**: Maintainer accepts ADR (Status: Accepted); `grep -c "Provider" docs/adr/0008-*.md` ≥10; no product code touched (`git diff --stat` shows only `docs/adr/`).
**Evidence/validation proof**: Store the acceptance note + `git log -1 -- docs/adr/0008-*.md` output in the PR description; pass = ADR merged with Accepted status and decision-log D-02/D-10 cross-links present. Fixture: none.
**Visual testing**: None (no UI). Reviewer renders the ADR Markdown at 100-col terminal width; pass = no table wider than 4 columns, all links resolve (`markdown-link-check` or manual click).
**Files to touch**: [do../adr/0008-terminology-charter.md](../adr/0008-terminology-charter.md) (new), [docs/adr/README.md](../adr/README.md) (index row).
**Research basis**: [decision-log.md](../review/decision-log.md#L21) D-02/D-10; pass2/01 §4.1–4.2 rename map; pass2/08 §3.4 ADR table; [0007-instance-centric-information-architecture.md](../adr/0007-instance-centric-information-architecture.md#L38).
**Handover note**: Next agent must not start AJ-E102 until this ADR is Accepted. Trap: pass2/01 once proposed "ADR-0014" — that numbering is superseded by pass2/08 §3.4 (0008); do not create 0014.

### AJ-E102 Phase A — UI copy sweep to Provider/Account/Instance | Status: DONE-EVIDENCED (zcode-primary, 2026-10-13) | Priority P0 | Effort M | Depends: AJ-E101
**Goal**: Every user-visible string says Provider/Account/Instance (and "Launch settings" for Profile), while routes, props, and IPC names stay aliased and working.
**User story**: As a first-run user (P1 Multi-Maxer), I want "Add account" and Provider names I recognise so I never have to learn what a "harness" or "profile" is.
**Requirements**:
1. The rail, headers, modals, empty states, errors, and tooltips in `ui/src/App.tsx`, `ui/src/components/WorkflowRail.tsx`, `ui/src/screens/Profiles.tsx`, `ui/src/screens/NewSession.tsx`, `ui/src/screens/OnboardingWizard.tsx`, `ui/src/screens/WorkPanels.tsx`, and `ui/src/screens/ReadyHome.tsx` SHALL use Provider/Account/Instance; "harness" SHALL appear only inside a Provider tooltip naming the binary (e.g. "Claude Code (claude-code)").
2. "Add profile" → "Add account", "Launch with profile" → "Start on [Account label]", "Pick a profile" → "Pick an account", "No profiles yet" → "No accounts yet — add an account first" SHALL be applied verbatim; internal identifiers (`ProfileInfo`, `profilesList`) SHALL NOT be renamed in this phase.
3. The Instances tab SHALL be kept (reversing deep-01 B-08); Instances = live/recent occurrences, Sessions = conversations; both tabs SHALL keep `⌘1..⌘3` shortcuts and gain `aria-controls`/arrow-key tab behaviour if touched.
4. Every renamed string SHALL keep a text equivalent (icon + text, never colour alone) per WCAG 2.2 AA 1.4.1/4.1.3, and status changes SHALL use `role="status"` where the component already announces.
5. A UI string allow-list test SHALL fail CI if /\bharness\b/i or /\bprofile\b/i appears in user-visible JSX text (excluding comments, `aria-label` legacy aliases documented in the ADR, and the Launch settings sheet title).
**UI/UX reasoning**: Nielsen NN-g #2 Match between system and real world + #4 Consistency; Apple HIG "use people's familiar words". Exact states: default / hover / focus-visible (2px outline) / disabled ("Sign in first" chip) / empty / error MUST all carry the new nouns — renaming only the happy path fails review. Shortcut: no shortcut letters change in Phase A.
**Performance budget**: Copy-only; bundle delta ≤+2 KB gzipped, rail render ≤300 ms at 50 rows, no new IPC calls (0 added, poll interval unchanged).
**Quality gates**: `pnpm -C ui build` green; string-guard test green; zero `window.alert` introduced; existing Playwright/smoke flows still pass unmodified (alias proof).
**Evidence/validation proof**: Run `pnpm -C ui build && grep -RniE "add profile|launch with profile" ui/src --include="*.tsx"` → 0 hits (store output in PR). Live-dogfood: launch app, screenshot Accounts + New Session + Onboarding; pass = no "harness/profile" visible. Evidence stored under PR attachments + `docs/qa/` note.
**Visual testing**: Capture 1280×800 and 390×844 (light only, current theme): (a) rail Projects/Instances/Sessions, (b) Accounts empty + populated, (c) New Session picker, (d) Onboarding scan results. Compare pre/post side-by-side; pass = only copy differs, layout shift ≤2 px. A11y: axe-core 0 violations on (b)/(c), keyboard-only tab traversal recorded.
**Files to touch**: [ui/src/App.tsx](../../ui/src/App.tsx), [ui/src/components/WorkflowRail.tsx](../../ui/src/components/WorkflowRail.tsx), [ui/src/screens/Profiles.tsx](../../ui/src/screens/Profiles.tsx), [ui/src/screens/NewSession.tsx](../../ui/src/screens/NewSession.tsx), [ui/src/screens/OnboardingWizard.tsx](../../ui/src/screens/OnboardingWizard.tsx), [ui/src/screens/WorkPanels.tsx](../../ui/src/screens/WorkPanels.tsx).
**Research basis**: pass2/01 §4.2 UI rows; pass2/08 §3.4 Phase A; pass2/00 §3; deep/01 B-08 (reversed) and B-11 row grammar.
**Handover note**: Do not rename TS types or bridge functions here — AJ-E107 owns that. Trap: `SessionInfo.ts:3` collapse (Instance rendered as filtered Session) is a data bug, not copy; flag it for AJ-E104, do not paper over it with labels.

### AJ-E103 Phase B1 — Domain aliases ProviderId and ProviderAdapter | Status: DONE-EVIDENCED (zcode-primary, 2026-10-13) | Priority P0 | Effort M | Depends: AJ-E101
**Goal**: Introduce `ProviderId`/`ProviderAdapter` as the canonical domain names with `HarnessId`/`HarnessAdapter` as deprecated aliases, without breaking serialised data or external adapters.
**User story**: As an adapter author, I want my existing adapter to keep compiling for one minor version so the rename never forces a flag-day rewrite.
**Requirements**:
1. `crates/juggler-core/src/model.rs:49` SHALL gain `pub type ProviderId = HarnessId` (then invert at removal: canonical struct renamed, alias kept) with `#[deprecated]` on the legacy name only after the canonical name exists; wire values (`claude-code`) SHALL be identical through both names.
2. `Account.harness` (model.rs:149) SHALL gain a `provider()` accessor returning `ProviderId`; the field SHALL be renamed to `provider` only with `#[serde(alias = "harness")]` where the type is serialised, and storage code SHALL read both column names during the v4→v5 window (AJ-E106).
3. `crates/juggler-adapter/src/adapter.rs` SHALL expose `ProviderAdapter` as the canonical trait name with `pub use` alias `HarnessAdapter`; `crates/juggler-adapter/src/lib.rs` and `registry.rs` SHALL register/resolve by `ProviderId` while accepting `HarnessId` at the boundary.
4. `crates/juggler-core/src/lib.rs` re-exports SHALL export both names; `cargo-semver-checks` SHALL pass (additive only, no removed items).
5. No crate directory SHALL be renamed (`juggler-adapter` stays); doc-comments SHALL say Provider, with one line "formerly HarnessId" for searchability.
**UI/UX reasoning**: Not user-facing; applies Nielsen NN-g #4 at the API surface — one canonical name, one documented alias, no silent third spelling. Error messages mentioning the provider SHALL use the display name ("Claude Code"), never the raw id, per pass2/01 §4.2 tooltip rule.
**Performance budget**: Zero-cost aliases; `cargo build --release` time delta ≤+2%, binary size delta ≤+0.5%, no runtime allocation change (0 ms hot path).
**Quality gates**: `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo-semver-checks` (or `cargo public-api` diff = additions only) all green.
**Evidence/validation proof**: Fixture: serialise an `Account` with old field/column names, deserialise via new names, assert equality; run `cargo test -p juggler-core storage_roundtrip` (1000 proptest cases) — pass = 0 failures, stored JSON strings diff = 0 bytes. Store `cargo public-api` diff in PR.
**Visual testing**: None. Pass criteria: API diff artifact shows additions only; no screenshot required (note "N/A — API surface" in PR).
**Files to touch**: [crates/juggler-core/src/model.rs](../../crates/juggler-core/src/model.rs#L49), [crates/juggler-core/src/lib.rs](../../crates/juggler-core/src/lib.rs), [crates/juggler-adapter/src/lib.rs](../../crates/juggler-adapter/src/lib.rs), [crates/juggler-adapter/src/adapter.rs](../../crates/juggler-adapter/src/adapter.rs), [crates/juggler-adapter/src/registry.rs](../../crates/juggler-adapter/src/registry.rs).
**Research basis**: pass2/01 §4.2 Domain/Adapter rows; pass2/08 §3.4 Phase B ("serialisation-compatible aliases, one minor version"); [crates/juggler-adapter/src/adapter.rs](../../crates/juggler-adapter/src/adapter.rs) semver note.
**Handover note**: Removal of the deprecated aliases is explicitly out of scope (needs its own major-version task). Trap: `id_newtype!` macro generates `Debug` as `HarnessId(...)` — update the macro invocation, not hand-written impls, or snapshot tests will churn.

### AJ-E104 Phase B2 — Split Instance from Session (InstanceRecord + ConversationId) | Status: DONE-EVIDENCED (zcode-primary, 2026-10-13) | Priority P0 | Effort L | Depends: AJ-E103
**Goal**: Make Instance a real domain entity (occurrence) with the conversation (Session) inside it, ending the shipped collapse where Instances are merely filtered Sessions.
**User story**: As a daily user (P1), I want an Instance row to mean "this tool running here under this Account" so switching, resources, and leftovers attach to the right thing.
**Requirements**:
1. `crates/juggler-core/src/model.rs:389` `SessionRecord` SHALL be renamed `InstanceRecord` (alias `SessionRecord` kept one minor version) and SHALL gain `conversation: ConversationId` (new newtype beside `SessionId` at model.rs:59); `SessionId` SHALL be retained as the conversation identifier type or aliased to it — pick one in ADR-0008 amendment and apply consistently.
2. `InstanceKind` (model.rs:325, `terminal`/`desktop`) SHALL stay on the Instance; `SessionStatus` (model.rs:188) wire strings (`running`, `waiting_user`, …) SHALL NOT change; any `InstanceStatus` naming SHALL be an alias.
3. `SessionRecord.profile` (model.rs:395) SHALL become `launch_settings: LaunchSettingsId` only together with AJ-E105, or stay `profile` behind an accessor until then — the two renames SHALL NOT land half-applied in one PR.
4. Storage `sessions` reads SHALL map to `InstanceRecord` without changing row values; the `kind` column added in schema v3 (ADR-0007) SHALL be the migration anchor, not a new column.
5. The UI view-model SHALL expose Instance and Conversation as distinct concepts (Instance row → conversation inside) even before AJ-E107 renames the TS types; no screen SHALL render "Instance" from a bare Session filter after this task.
**UI/UX reasoning**: Nielsen NN-g #1 Visibility of system status + pass2/04 indicator grammar — the Instance row carries Provider/Account/model/quota, the Conversation carries transcript state; conflating them hides which is running. Apple HIG clarity: one object, one row. Loading/empty/error states: Instance with no conversation yet SHALL show "Starting…" (status Starting), never a blank Session.
**Performance budget**: Model change only; list Instances ≤300 ms at 200 rows, serialisation round-trip ≤5 ms/record, memory ≤+1 MB at 1k records vs baseline.
**Quality gates**: `cargo test --workspace` green; new unit tests for ConversationId/InstanceRecord mapping; `storage_roundtrip` proptest extended (not weakened) and green.
**Evidence/validation proof**: Round-trip old DB/fixtures: open a v4 fixture DB (pre-rename rows), read as InstanceRecord, write, re-read — values equal, `PRAGMA user_version` unchanged by this task alone. Live-dogfood: launch one Terminal Instance, confirm rail Instance row + its conversation render; evidence = test output + dogfood note in PR.
**Visual testing**: Capture rail Instances list + Instance panel at 1280×800 (running / waiting / finished states). Pass = Instance vs Conversation visually distinguishable (kind badge + conversation preview), axe 0 violations, no layout regression >2 px vs AJ-E102 shots.
**Files to touch**: [crates/juggler-core/src/model.rs](../../crates/juggler-core/src/model.rs#L389), [crates/juggler-core/src/storage/sessions.rs](../../crates/juggler-core/src/storage/sessions.rs), [crates/juggler-core/src/storage/mod.rs](../../crates/juggler-core/src/storage/mod.rs), [ui/src/types/generated/SessionInfo.ts](../../ui/src/types/generated/SessionInfo.ts) (regenerated, see AJ-E107).
**Research basis**: ADR-0007 decision 1 (Instance first-class, Session inside); pass2/01 §4.1 Instance row + conflict #4; pass2/00 §3 (`SessionInfo.ts:3` collapse = load-bearing bug).
**Handover note**: If AJ-E105 has not landed, keep the `profile` field name and add only the accessor — do not invent a half-renamed `launch_settings` column. Trap: transcripts' `native_id` belongs to the conversation, not the Instance; moving it breaks handover lookup.

### AJ-E105 Profile demotion — Launch settings (UI) and LaunchRecipe (domain) | Status: DONE-EVIDENCED (zcode-primary, 2026-10-13) | Priority P0 | Effort M | Depends: AJ-E104
**Goal**:
Demote `Profile` from a rail noun to per-Account "Launch settings"
inside the launch sheet, and rename the domain type to `LaunchRecipe`
(internal) with a serialisation-compatible alias.
**User story**:
As a new user (P2 Work/Life Splitter), I want to pick an Account and
press Start without inventing a named "profile" first.
**Requirements**:
1. `Profile` (model.rs:173), `ProfileId` (model.rs:57), and
   `SettingsOverlay` (model.rs:161) SHALL be renamed canonically to
   `LaunchRecipe`, `LaunchRecipeId`, and retained `SettingsOverlay`;
   `pub type Profile = LaunchRecipe` alias SHALL remain one version.
2. The stored JSON key `settings_json` and the `profiles` table values
   SHALL NOT change in this task; only Rust identifiers and the UI
   noun change (table rename is AJ-E106).
3. UI: Profiles screen (`ui/src/screens/Profiles.tsx`) SHALL be
   retitled around Accounts; the per-Account launch sheet SHALL show
   "Launch settings" (model + env + default cwd) as an expandable
   section, collapsed by default, with "Start on [Account]" primary.
4. Copy SHALL never use "profile" to mean Account; the string guard
   from AJ-E102 SHALL be tightened so `profile` survives only inside
   the Launch settings sheet and migration notes.
5. The default recipe ("Main") created on account create/import
   (commands.rs account flow) SHALL be auto-derived with zero
   user-named settings (anti-configuration budget, pass2/01 R6).
**UI/UX reasoning**:
Nielsen NN-g #8 Aesthetic and minimalist design + Apple HIG
progressive disclosure: launch settings are rarely edited, so they
collapse; the Account is the decision. Exact copy: sheet title
"Launch settings", helper "Applies when you start on this account.",
primary button "Start", secondary "Edit launch settings".
States: collapsed (default) / expanded / dirty (Save + Reset) /
error (inline `role="alert"`, no `window.alert`). Shortcut: `Enter`
starts, `Esc` closes without saving.
**Performance budget**:
Launch sheet open ≤150 ms; save round-trip ≤100 ms local;
zero added IPC; settings JSON size unchanged (±0 bytes values).
**Quality gates**:
`cargo test --workspace` + `pnpm -C ui build` green; string-guard
test green; no new `window.alert` (grep count stays 0 added).
**Evidence/validation proof**:
Round-trip: deserialise a pre-rename `Profile` JSON fixture via
`LaunchRecipe`, serialise, diff values = 0 bytes difference.
Dogfood: create Account → Start with collapsed Launch settings →
terminal launches; edit model → relaunch → new model honoured.
Store test output + dogfood checklist in PR; pass = both green.
**Visual testing**:
Capture launch sheet 1280×800 + 390×844 in collapsed, expanded,
dirty, and error states. Pass = "Profile" absent as a noun,
"Launch settings" present once, primary action visible without
scrolling at 390×844, axe-core 0 violations, focus order
Account → Start → Edit launch settings.
**Files to touch**:
[crates/juggler-core/src/model.rs](../../crates/juggler-core/src/model.rs#L173),
[crates/juggler-core/src/storage/profiles.rs](../../crates/juggler-core/src/storage/profiles.rs),
[ui/src/screens/Profiles.tsx](../../ui/src/screens/Profiles.tsx),
[crates/juggler-app/src/commands.rs](../../crates/juggler-app/src/commands.rs).
**Research basis**:
D-10 (decision-log.md:97); pass2/01 §4.1 conflict #3 + §4.2;
pass2/08 §3.4 ("Profile survives only in settings-overlay contexts").
**Handover note**:
Trap: cc-switch/Codex call an account a "profile" — import flows
must translate that word to Account on entry and never echo it
back. If AJ-E106 has landed first, update the table name here too;
never leave type/table names disagreeing in one release.

### AJ-E106 DB schema v5 — provider_id / instances / launch_recipes migration | Status: DONE-EVIDENCED (zcode-primary, 2026-10-13) | Priority P0 | Effort M | Depends: AJ-E105
**Goal**:
Rename storage identifiers to the canonical nouns in one versioned
migration (v5) that upgrades every existing user database in place
without changing a single stored value.
**User story**:
As an existing user, I want my Accounts, history, and Instances to
survive the rename exactly, with no re-onboarding.
**Requirements**:
1. A new `V5` migration SHALL be appended to `MIGRATIONS` in
   `crates/juggler-core/src/storage/mod.rs` (currently V1–V4):
   `accounts.harness` → `provider_id`, `sessions` → `instances`
   (+ `conversation_id`), `profiles` → `launch_recipes`,
   `sessions.profile_id` → `launch_recipe_id`.
2. The migration SHALL use `ALTER TABLE … RENAME` (SQLite ≥3.25)
   inside the existing per-version transaction; `PRAGMA user_version`
   SHALL advance 4→5 exactly once; failure SHALL roll back to v4.
3. Stored values SHALL NOT be rewritten: provider ids (`claude-code`),
   status strings, `kind` (`terminal`/`desktop`), JSON blobs, and
   timestamps SHALL be byte-identical before/after (verified by dump).
4. Repositories (`accounts.rs`, `profiles.rs`, `sessions.rs`) SHALL
   query only the new names after v5; a v4 database SHALL auto-migrate
   on `Storage::open` with no user action and no data-loss path.
5. Downgrade SHALL be documented as unsupported (forward-only policy,
   QUALITY.md §1); opening a v5 DB with a v4 build SHALL yield the
   existing `SchemaTooNew` error, never silent corruption.
**UI/UX reasoning**:
No UI; applies Nielsen NN-g #9 (help users recover from errors) at
the data layer: migration failure surfaces the existing plain-language
`NotADatabase`/`SchemaTooNew` errors, never a raw SQLite string.
First launch after upgrade SHALL show no dialog (silent success);
evidence of migration lives in logs, not a modal.
**Performance budget**:
Migration ≤500 ms for 10k sessions / 1k accounts on M-series SSD;
DB file growth ≤+5% (rename only, no rebuild); open-time regression
≤+10 ms p95; no poll or background rewrite after migration.
**Quality gates**:
`cargo test -p juggler-core` green including new v4→v5 tests;
up/down discipline per EPICS.md:44 (migrate-up asserted, forward-only
documented); clippy `-D warnings` green.
**Evidence/validation proof**:
Strict round-trip old DB/fixtures: build a v4 fixture DB with the
pre-rename code (or checked-in fixture), dump (`sqlite3 .dump`),
migrate, dump again — pass = values diff 0 (only identifiers differ),
row counts equal, FK integrity `PRAGMA foreign_key_check` = 0 rows.
Proptest `storage_roundtrip` SHALL run ≥1000 cases against v5.
Store both dumps + test log in PR; that artifact is the proof.
**Visual testing**:
None (storage). Pass criteria recorded as dump diffs above;
note "N/A — storage migration" in PR with the fixture DB hash.
**Files to touch**:
[crates/juggler-core/src/storage/mod.rs](../../crates/juggler-core/src/storage/mod.rs),
[crates/juggler-core/src/storage/accounts.rs](../../crates/juggler-core/src/storage/accounts.rs),
[crates/juggler-core/src/storage/profiles.rs](../../crates/juggler-core/src/storage/profiles.rs),
[crates/juggler-core/src/storage/sessions.rs](../../crates/juggler-core/src/storage/sessions.rs),
[crates/juggler-core/tests/storage_roundtrip.rs](../../crates/juggler-core/tests/storage_roundtrip.rs).
**Research basis**:
Pass2/01 §4.2 DB row + §4.3 (ride the v5 migration, one payment);
pass2/08 §3.1 §3 note; storage forward-only policy in mod.rs header.
**Handover note**:
Coordinate with AJ-E104: `conversation_id` values backfill from the
existing session id — do not mint new ids that orphan transcript
links. Trap: SQLite RENAME rewrites FK references automatically;
verify `sessions→profiles` FK still cascades after both renames.

### AJ-E107 Bridge and generated-types rename (commands + TS) | Status: DONE-EVIDENCED (zcode-primary, 2026-10-13) | Priority P1 | Effort M | Depends: AJ-E106
**Goal**:
Rename the app-command and generated-TS surface to Provider/Account/
Instance/LaunchRecipe while keeping every IPC payload wire-compatible
for one release.
**User story**:
As a UI developer, I want `ProviderInfo`/`InstanceInfo` types so new
code never has to translate "harness" mentally again.
**Requirements**:
1. `crates/juggler-app/src/commands.rs` DTOs SHALL be renamed:
   `HarnessInfo` → `ProviderInfo`, `ProfileInfo` →
   `LaunchRecipeInfo`, `SessionInfo` → `InstanceInfo` (with
   `conversation_id`); old `#[serde(alias)]` field names (`harness`)
   SHALL deserialise for one version.
2. Tauri command names SHALL gain canonical names
   (`providers_list`, `launch_recipes_list`, `instances_list`) with
   old names (`harnesses_list`, `profiles_list`, `sessions_list`)
   registered as aliases for one version; no command SHALL change
   its argument or return values, only identifiers.
3. Generated TS in `ui/src/types/generated/` SHALL be regenerated
   via the existing ts-rs export (never hand-edited); new files
   `ProviderInfo.ts`/`InstanceInfo.ts`/`LaunchRecipeInfo.ts` SHALL
   appear and old files SHALL re-export them as deprecated aliases.
4. `ui/src/lib/bridge.ts` SHALL expose canonical function names with
   legacy aliases; `ui/src/App.tsx` call sites SHALL use canonical
   names only after regeneration.
5. Adapter implementations (`claude_code.rs`, `fake.rs`) SHALL
   implement `ProviderAdapter`; the `HarnessAdapter` alias SHALL
   keep third-party adapters compiling (AJ-E103 contract).
**UI/UX reasoning**:
Invisible to users; Nielsen NN-g #4 at the code boundary. Any error
crossing the bridge SHALL carry the Provider display name + Account
label (never a raw `HarnessId` debug string) per WCAG 2.2 AA 4.1.3
status-message practice already used in the UI.
**Performance budget**:
IPC payload size ±0 bytes in values; bridge call latency ≤+1 ms;
`pnpm -C ui build` time ≤+5%; no extra polling introduced.
**Quality gates**:
`cargo test --workspace`, `pnpm -C ui build`, and the ts-rs export
test (`cargo test -p juggler-app`) all green; `git diff` on generated
files shows rename + alias only, no value-shape change.
**Evidence/validation proof**:
Fixture: replay a captured pre-rename IPC JSON payload through the
new deserialisers — pass = identical structs, 0 errors. Round-trip
old DB via AJ-E106 fixture then list via new commands — counts equal.
Store payload fixture + command outputs in PR.
Live-dogfood step: launch, list Providers/Accounts/Instances,
start one Instance through the renamed bridge.
**Visual testing**:
Regression capture only: Accounts + Instances + New Session at
1280×800 post-rename vs AJ-E105 baseline. Pass = pixel-diff limited
to intended copy, axe 0 violations, keyboard launch flow intact.
**Files to touch**:
[crates/juggler-app/src/commands.rs](../../crates/juggler-app/src/commands.rs),
[ui/src/lib/bridge.ts](../../ui/src/lib/bridge.ts),
[ui/src/types/generated/HarnessInfo.ts](../../ui/src/types/generated/HarnessInfo.ts),
[crates/juggler-adapters/src/claude_code.rs](../../crates/juggler-adapters/src/claude_code.rs),
[crates/juggler-adapters/src/fake.rs](../../crates/juggler-adapters/src/fake.rs).
**Research basis**:
Pass2/01 §4.2 (Adapter API + DB rows); pass2/08 §3.4 Phase B;
ts-rs header in every `ui/src/types/generated/*.ts` ("Do not edit").
**Handover note**:
Never hand-edit generated TS — change the Rust DTO and re-export.
Trap: `LaunchResult.session_id` is wire data; rename the Rust field
only with `#[serde(alias = "session_id")]` or the UI breaks silently.

### AJ-E108 Docs sweep and terminology lint gate | Status: DONE-EVIDENCED (zcode-primary, 2026-10-13) | Priority P1 | Effort S | Depends: AJ-E107
**Goal**:
Bring SPEC, PROPOSAL, and EPICS to the canonical nouns and add a
lint gate so legacy user-facing terms cannot creep back.
**User story**:
As a contributor, I want the specs to use the same words as the
product so I stop translating while reading.
**Requirements**:
1. `docs/SPEC.md` §§3, 4, 9, 12.1 SHALL be swept: harness→Provider,
   profile→Launch settings/LaunchRecipe (internal contexts only),
   session(board)→Instance where occurrence is meant; Session SHALL
   remain only for the conversation. `docs/SPEC.md:426`-class vendor
   compliance sentences SHALL be left untouched.
2. `docs/PROPOSAL.md` §§1, 6 and `docs/plan/EPICS.md` task names
   SHALL adopt Provider/Account/Instance and the D-09 tagline/
   subtitle; one glossary box mapping legacy→canonical SHALL be
   added to SPEC §3 and linked from PROPOSAL and EPICS.
3. Historical evidence SHALL NOT be rewritten: `docs/review/**`,
   `docs/research/**`, and prior ADRs stay verbatim; the sweep SHALL
   list them as explicitly out of scope in the PR.
4. A docs lint (script or CI step) SHALL fail on new user-facing
   `harness`/`profile`-as-account in `docs/SPEC.md`, `docs/PROPOSAL.md`,
   `docs/plan/EPICS.md`, and `ui/src` JSX text, with an allow-list
   for compliance `vendor`, adapter-internal notes, and this epic.
5. CHANGELOG SHALL gain an "Renamed" entry naming old→new terms and
   stating "stored data and wire IDs unchanged; old DBs migrate
   automatically (schema v5)".
**UI/UX reasoning**:
Nielsen NN-g #4 Consistency across product and documentation;
WCAG 2.2 AA 3.2.3 consistent identification — the same action keeps
the same label in app, spec, and help. Glossary definitions reuse
the ≤160-character ADR-0008 wording verbatim (no paraphrase drift).
**Performance budget**:
Docs only; lint step ≤10 s in CI on this repo size; 0 ms runtime,
0 MB shipped impact.
**Quality gates**:
Lint gate green on the swept tree and proven to fail on a seeded
violation (test commit reverted); all intra-repo Markdown links
resolve; `wc -l` and spelling pass per repo conventions.
**Evidence/validation proof**:
Run the lint script + link check, store output in PR; pass = exit 0
on swept tree, exit non-zero on the seeded `harness` fixture.
Dogfood: a new contributor (or reviewer) finds "how Accounts relate
to Providers" from SPEC §3 glossary in ≤2 clicks — record the path.
**Visual testing**:
Render SPEC §3 + PROPOSAL §1 + EPICS in Markdown preview at
100-col width. Pass = glossary table ≤4 columns, no truncated
definitions, screenshots attached for glossary + one swept section.
**Files to touch**:
[docs/SPEC.md](../SPEC.md),
[docs/PROPOSAL.md](../PROPOSAL.md),
[docs/plan/EPICS.md](../plan/EPICS.md),
[CHANGELOG.md](../../CHANGELOG.md).
**Research basis**:
Pass2/01 §4.2 Docs row + §4.3; pass2/08 §3.1–3.3 edit order, §5 D2;
pass2/00 §5.2 (SPEC/PROPOSAL/EPICS update order after decisions).
**Handover note**:
This task closes E1. Alias removal (HarnessId/HarnessAdapter/Profile/
old command names) needs a separate major-version epic — do not
smuggle it in. Trap: a global find-replace will corrupt compliance
`vendor` sentences and historical quotes; sweep by section, by hand.



