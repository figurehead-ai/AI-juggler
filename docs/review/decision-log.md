# AI Juggler — Decision Log

> Status: **Final** — marked by the maintainer's instruction on
> 2026-10-13 ("Implement all as spec-ed out, I already approved those
> decisions with the other agent"), which also authorises Stage 1
> (documents) and the staged implementation that follows per D-15.
> Source evidence: `docs/review/AI-Juggler-Maximum-Depth-Report-and-Proposal.md`,
> `docs/review/AI-Juggler-Expected-Workflow-Second-Pass.md`, and
> `docs/internal/expected-workflow.md`.

## Settled decisions

### D-01 Product thesis centre — SETTLED
- Decision: Re-centre from quota-first to **Switch → See → Clean**.
  Unlimited reversible switching is the core promise; the right-hand
  inspector is the daily surface; verified process cleanup is the
  trust proof; quota is a trigger/input, not the headline.
- Consequences: success measured by time-to-first-switch, A→B→A
  continuity, and zero false cleanup kills, not dashboard views.
  Roadmap resequences accordingly (see Second Pass Part 08 §4).

### D-02 Terminology depth — SETTLED
- Decision: **Full rename, phased** to Providers / Accounts /
  Instances. UI first, then domain types with serialisation-compatible
  aliases, then docs. Provider = HarnessId, Account stays, Instance =
  SessionRecord + kind; Session retained only as the conversation
  inside an Instance; Profile demoted to launch-settings wording.
- Consequences: ADR-0008 required; code rename is phased, never
  big-bang; external tools' use of "profile = account" must never be
  imported into Juggler's meaning.

### D-03 Switching interaction — SETTLED
- Decision: **Hybrid**. Same-Provider switch with proven fidelity
  runs in one click with an undo window; cross-Provider,
  low-fidelity, or diverged switches open a brief wizard showing
  what will copy, translate, regenerate, or drop.
- Consequences: the daily quota-wall switch stays fast; lossy
  switches can never happen silently.

### D-04 Fidelity gate under beta — SETTLED
- Decision: **Ship all switching in beta immediately**, with an
  in-depth validation and research programme as a mandatory,
  parallel workstream (fixture matrix + live-binary dogfood per
  Provider pair). Beta status is the disclosure vehicle: fidelity
  state per pair must still be shown honestly (validated vs
  unvalidated), and no pair graduates out of beta labelling until
  its validation matrix passes.
- Consequences: no feature is held for proof, but validation debt
  is tracked as beta-exit criteria, not optional polish.

### D-05 Switch-back divergence — SETTLED
- Decision: **Resume original plus one delta**. The original
  Account's native session resumes; work done on the interim
  Account arrives as a single summarised delta turn with the
  portable context pack attached. Copy-back over the original
  session is prohibited; independent divergence is surfaced,
  never silently overwritten.
- Consequences: the original session stays canonical; lineage
  records the excursion as a Switch Graph cycle.

### D-06 Right Inspector scope — SETTLED
- Decision: **Full inspector, honest fields**. All six tabs
  (Overview, Usage, History, Quality/Performance, Processes,
  Actions) exist for every selection type (Session, Provider,
  Account, Instance, Project) from the start. A field appears
  only when actually measurable; otherwise it shows unavailable
  with the reason, never a dressed-up estimate.
- Consequences: the inspector is the stable daily home while
  data sources mature behind it.

### D-07 Quality definition — SETTLED
- Decision: **Measurable signals only**. Error/retry rate,
  stuck/waiting state, context-window use, tokens and cost per
  completed task where known, and process health, each labelled
  with its source. No composite quality score until dogfood data
  shows which signals predict a good session.
- Consequences: Quality/Performance tab ships in beta without an
  invented 0–100 score.

### D-08 Automatic cleanup scope — SETTLED
- Decision: **Notify-only default, Clean-safe opt-in per Account**.
  Juggler detects and verifies ownership and alerts; it kills
  automatically only for Accounts the user has opted in, only
  verified-owned processes, every kill audit-logged. This is a
  narrow, written amendment to the passive-automation invariant.
- Consequences: ADR-0010 must state the tier model; Tier wording
  in SPEC §5 changes only as this ruling allows.

### D-09 Positioning sentence — SETTLED
- Decision: **A as tagline + B as subtitle**. Tagline: "Start
  anywhere. Switch when you hit the wall. Never lose the thread —
  or leave a zombie behind." Subtitle: "One console for every
  Provider, Account, and Instance you juggle." Option C retired
  to SEO/docs metadata only.
- Consequences: "zombie" needs a plain-language glossary on first
  use in product copy.

### D-10 "Profile" wording — SETTLED
- Decision: **Demote to "Launch settings"**. Profile disappears
  as a rail noun; it lives only as per-Account launch settings
  inside the launch sheet. The word must never mean Account in
  Juggler copy (external tools' usage is explicitly not imported).
- Consequences: UI copy sweep in naming Phase A; ADR-0008 records
  the demotion.

### D-11 Cross-Provider launch claim — SETTLED
- Decision: **Labelled beta**. Cross-Provider switching is
  available immediately in beta; every such switch shows its
  fidelity badge and the wizard lists what will translate or
  drop. The claim is "available in beta", never "lossless".
- Consequences: consistent with D-04; marketing copy must carry
  the beta + fidelity qualifier.

### D-12 Subscription-expiry data source — SETTLED
- Decision: **User-entered, optional**. The user may add a
  renewal date per Account; the indicator warns as it approaches.
  With no date entered, expiry shows as unknown — never inferred
  or guessed.
- Mandatory research item (before assuming no source exists):
  in-depth investigation of whether any Provider APIs/SDKs expose
  billing/renewal dates (Claude, Codex/OpenAI, ZCode, Gemini and
  successors). Any source found must be sanctioned, read-only,
  and verified live before use; unproven sources stay "unknown".
- Consequences: expiry ships in beta on manual entry; automated
  population is a research-gated upgrade, not a promise.

### D-13 Fleet timing — SETTLED
- Decision: **Fleet ships immediately with the launch**,
  architected and built from the start so multi-machine support
  integrates cleanly rather than being retrofitted. This
  overrides the schema-only recommendation in Part 08 §5 D6.
- Consequences: Fleet is a Phase-0 architecture constraint, not
  a post-1.0 add-on. Local switching, inspector, spawn-ledger and
  cleanup designs must be fleet-shaped from day one (identity,
  sync/transport, remote process ownership, offline behaviour and
  security model all become beta-exit criteria). Scope and
  roadmap estimates must be re-planned around this; the local
  loop's safety gates still apply per machine.

### D-14 Pack redaction policy — SETTLED
- Decision: **Always the user's choice, remembered**. Secret
  handling in the portable context pack is governed by a user
  setting, never a hard-coded product default imposed on the
  user. The choice is remembered globally and can be overridden
  per Provider (global policy + per-Provider override).
- Consequences: the pack manifest must still list what was
  detected and how the active policy treated it, and the active
  policy for the destination Provider must be visible before a
  switch executes. First-run setup must ask the user to choose
  the global policy explicitly; per-Provider overrides live in
  Provider settings.

### D-15 Implementation scope contract — SETTLED
- Decision: **Stage 1, documents only**. This stage rewrites
  SPEC, PROPOSAL and EPICS from the settled decisions, drafts
  ADR-0008 to ADR-0012 plus a Fleet architecture ADR, and
  records the mandatory Provider billing-API research (D-12).
  No product code changes in this stage.
- Done means: the documents agree with each other and with this
  decision log, and the maintainer accepts them explicitly.
  Code implementation follows only under a separate approval;
  accepting this log alone does not authorise a build.
- Out of scope for Stage 1: product code, tests, runtime
  behaviour changes, releases, and any later implementation
  stage, each of which returns for its own approval.

### D-16 Dual theme — Light + Dark — SETTLED (2026-10-13)
- Decision: the system ships **two themes, Light and Dark**. Dark
  remains the default; `System` (follow the OS) is offered; the
  toggle lives in Settings and cycles with `⌘⇧L`. Both themes are
  first-class: every screen, state, chart, terminal colour, and
  accessibility affordance (contrast, focus rings, WCAG 1.4.1
  text+icon redundancy) must pass in both; the token layer
  (`ui/src/styles.css` custom properties) is the single source, with
  no hard-coded colours allowed anywhere.
- Consequences: AJ-E401 (shell read model/indicators) and AJ-E403
  (density/legend) gain explicit light-theme states in their visual
  testing (every state captured in BOTH themes); the a11y gate
  (AJ-E804) runs in both themes; screen 26 (Settings) gains the
  Appearance section. Wireframe convention recorded in
  `docs/wireframes/README.md`.

### D-17 Screen capture permitted — vendored, audited init shim — SETTLED (2026-10-13)

- Decision: the maintainer granted screen capture ("Yes I am giving
  you permission for screen capture", 2026-10-13), unblocking
  AJ-E415's real capture path. The `screencapturekit` binding requires
  one `unsafe { sc_initialize_core_graphics() }` call at first use to
  avoid a first-capture SIGSEGV (reproduced and logged at
  `docs/qa/evidence/AJ-E415/real-capture-crash.log`); the workspace
  law `unsafe_code = "forbid"` stays ABSOLUTE for every existing
  crate. The carve-out is a NEW, isolated crate
  (`crates/juggler-scap-init`) that deliberately does NOT inherit the
  workspace lint baseline (`[lints] workspace = true` omitted) and
  contains exactly ONE `unsafe` block calling the binding's own
  public FFI export behind a `Once` — audited line-by-line in this
  decision, documented in the crate root, and load-bearing for
  nothing else.
- Consequences: `ParkedSource` is deleted; macOS previews use the
  real `ScreenshotSource` (init via the shim); Screen Recording
  permission denials still surface honestly as the `denied` phase;
  non-macOS stays `unsupported`. Screen Recording TCC prompts are
  expected on first use (the `NSScreenCaptureUsageDescription`
  Info.plist key shipped with the parked subset). Any future `unsafe`
  request requires its own recorded decision.

### D-18 Optional Figurehead account: two paths (desktop / cloud) — SETTLED (2026-10-17)

- Decision, in the maintainer's words: "What we have to do is always
  offer two paths: some features are available only on desktop, some
  on cloud, and some in both. If a user wants things easily managed
  via cloud, then they select that and they agree to it. Those that
  don't just don't create the account or log in, but they might miss
  out on some features." · "AI Juggler will have an optional account.
  The original rule is stale and will be updated." · "Let's only
  launch with free features."
- What it means:
  1. **The desktop path is complete on its own.** With no account, AI
     Juggler does everything it does today, and nothing it already
     does ever moves behind an account. With no account it makes
     **zero** calls to figurehead.ai.
  2. **The cloud path is an explicit choice.** The user connects an
     account and sees exactly which cloud features will run and what
     each one sends; every data-sharing feature has its own toggle.
  3. **Content is end-to-end encrypted.** Backups (and, later, config
     and transcripts) are encrypted on the Mac; the server stores
     ciphertext it cannot read. Vendor credentials and logins never
     leave the Mac, whatever the setting.
  4. **The automation policy still holds.** The app prepares; the
     user executes. Cloud features never perform a state-changing
     action without a click; uploading data the user turned on is not
     a state change.
  5. **Every cloud feature launches free.** Paid options may be
     considered later. The app reads the account's plan and limits
     from the server and never hard-codes which features are free or
     paid; no payment code goes in the app, and the app stays MIT.
- Consequences: ADR-0014 records the architecture and partly
  supersedes ADR-0004's "cloud relay / vendor backend — rejected
  outright" (account-based discovery is allowed; a relay stays out of
  scope until a separate decision). The "no cloud, no account" rules
  in README, PROPOSAL, SECURITY and SPEC §1/§8/§11 are updated in the
  same change. Work lives in epic E12 (`docs/tasks/E12-optional-account.md`).

### D-19 Opt-in usage statistics — SETTLED (2026-10-18)

- Decision: the maintainer approved the usage-statistics specification
  on 2026-10-16 (abuse protection and personal sync added 2026-10-17).
  AI Juggler may learn which features people use, only if each person
  opts in, and never anything about their work.
- What it means:
  1. **Opt-in, three modes:** `off` (the default: nothing is counted or
     stored, and no request is made), `local` (count on this Mac and
     show it, never send) and `share` (also send one report a week).
  2. **Aggregated on the Mac.** Only weekly counters exist; raw events
     are never stored or sent.
  3. **One report per ISO week (UTC)**, uploaded 0–144 h after the week
     closes, retried with backoff. Only a week fully covered by `share`
     is ever sent.
  4. **No identifier.** A report's single-use random id only lets the
     server drop duplicates; two reports from one Mac can't be linked.
  5. **Sampling** comes from a static config file and needs no id
     (a local salt that is never sent).
  6. **A closed schema** (`stats/schema.v1.json`, identical to the
     server's copy) lists every metric and value; anything else is
     rejected. Generated Rust enums with no string fields make free text
     impossible to send.
  7. **Crash reports** have their own setting (`never` by default, `ask`,
     `always`) and are scrubbed on the Mac.
  8. **Overrides:** `DO_NOT_TRACK`, `AI_JUGGLER_STATS=off|local`, CI and
     debug builds never upload, and building without the `stats`
     feature leaves it out entirely.
  9. **Consent versioning:** a new *category* of data asks again; new
     metrics within a category are listed in the CHANGELOG and
     `docs/usage-stats.md`.
  10. **Personal stats sync** (signed in, linked Mac, its own checkbox):
      the weekly report goes to the member's account *instead of* the
      anonymous endpoint, never both.
  11. **Abuse protection without identity:** a strict schema with caps,
      and an adaptive proof of work that stays off unless the service is
      under attack.
  12. **Never collected:** prompts, transcripts, code, paths, project or
      Account names, machine or user names, setting values outside the
      enums, error text; nothing read from, derived from or hashed from
      vendor credential or identity files.
- Consequences: ADR-0015 records the architecture. SPEC §1/§8, SECURITY
  and README say the same. The word "telemetry" keeps its E7 meaning (the
  status sync between paired Macs); this feature is "usage statistics".
  Work lives in epic E11 (`docs/tasks/E11-usage-statistics.md`).

### D-20 Three release channels: nightly → beta → stable — SETTLED (2026-10-19)

- Decision: AI Juggler ships on three update channels, and every change
  travels nightly → beta → stable, automated wherever it can be.
  - **Nightly**: built automatically from `main`, no approval. Newest
    work, least tested.
  - **Beta**: a nightly build promoted with the maintainer's approval;
    newer features, less tested than stable.
  - **Stable** (the main release): a beta build promoted with the
    maintainer's approval; well-tested features only.
- Consequences: each channel has its own update feed, and the app lets
  the user choose which one it follows (Settings). A promotion moves an
  already-built, signed build forward; nothing is rebuilt between
  channels. The approval steps are GitHub environment approvals, so
  they are recorded. One version stream: a nightly build gets the next
  version (`MAJOR.MINOR` from `Cargo.toml`; the patch starts at 0 for a
  new `MAJOR.MINOR` and goes up by one per build, as Semantic Versioning
  has it) and keeps it on every channel. *Amended 2026-10-24:* the patch
  was the nightly's run number, which never resets, so the first 0.1
  build came out as 0.1.18; it now counts from the release tags
  (`scripts/release_version.py`). Promotion to **stable** also
  needs the version to have been on beta and the beta gate (AJ-E807) to
  be signed: the gate's criteria are the exit from beta. Installs follow
  Beta until the first stable release. Supersedes the single beta
  channel of AJ-E1010 and the `0.x.0-beta.N` versioning of SPEC §15.9.

### D-21 Two visual-check modes: full sweep and core — SETTLED (2026-10-19)

- Decision: the visual regression check has two modes, chosen per
  change. **Full sweep** compares every gallery state; it is used when a
  screen is first built or rebuilt, and on demand. **Core** compares
  the approved baselines of the core screens on every UI pull request
  (AJ-E803 as built).
- Consequences: the full sweep needs no stored baselines for the whole
  gallery: it renders the base and the head of the change in the same
  pinned container and reports every state that differs, so the
  repository doesn't carry hundreds of reference images. The core mode
  stays a required check.

### D-22 Maintainer rulings on the E8 gates — SETTLED (2026-10-19)

- **Accessibility gate time** (AJ-E804): the gate checks every gallery
  state and runs per release candidate, split across four jobs. Its
  measured 10 minutes is accepted; the budget becomes ≤ 15 minutes.
- **Orphan detection** (AJ-E805): the budget reads "< 10 s after the
  grace window". The 10 s grace (AJ-E410) stands, so a well-behaved
  helper never shows as left behind.
- **Usage-stats consent in first-run setup** (AJ-E1105): shown on the
  policy step with equal weight and nothing pre-selected, and not
  required. No answer keeps statistics off. Flow (a) keeps its
  ≤ 5-click budget.
- **Dependencies before launch**: nothing has shipped, so every
  dependency, major versions included, is brought current now rather
  than deferred.
- **Repository**: the pre-move public repository is archived with a
  pointer here; `visual` is a required check; secret scanning and push
  protection are on for this public repository.
- **Bundle id**: `ai.figurehead.juggler`. It is the product's identity,
  independent of the signing team (Vuxtra LLC's Developer ID today), so a
  future Figurehead Apple account changes only the signer. No App ID is
  registered; the app uses no Apple capabilities.

### D-23 Editions: AI Juggler, AI Juggler Nightly and AI Juggler Dev side by side — SETTLED (2026-10-19)

- **Ruling** (maintainer): one stable install for daily use and one
  nightly install for trying new builds, on the same Mac, while
  development and agents build and test independently.
- **Decision**: three editions that share nothing. Each has its own
  bundle id (`ai.figurehead.juggler`, `….nightly`, `….dev`), app name,
  data folder, Keychain items, Fleet port, single-instance lock and icon.
  The edition follows the bundle identifier; a debug build is always Dev,
  and any unexpected identifier is Dev too, so a stray build can only
  reach the Dev data folder.
- **Channels**: AI Juggler chooses Stable or Beta; AI Juggler Nightly
  follows nightly only; AI Juggler Dev never updates. D-20's one version
  stream stands: the nightly run builds both apps from one commit, and
  promotion still points beta and stable at that version's AI Juggler
  build, never rebuilding it.
- **Consequences**: AI Juggler keeps its existing folder, Keychain items
  and port (no migration). `just dev` and `just app` build AI Juggler Dev.
  Nightly signing and notarizing take about twice as long. Each edition
  keeps its own copies of Accounts, so both can be used for real work;
  they share only the subscription's usage and the default ⌥Space
  shortcut.

### D-24 Channel requirements: when a build may move to beta and stable — SETTLED (2026-10-19)

- **Ruling** (maintainer): who merges or deploys doesn't matter; what
  matters is the point at which each channel is ready. Nightly gets
  changes as they land, beta needs real usage testing, stable is a
  deliberate release.
- **Research**: Chrome (Canary daily with only automated testing; a
  milestone ships to Stable after about four weeks on Beta, then a
  staged rollout), Firefox (crash rate per channel; release criteria
  reviewed for stability, performance, security, functionality), VS
  Code (daily Insiders; an endgame week before each Stable), Rust (a
  six-week nightly → beta → stable train; beta regressions fixed on the
  main branch first).
- **Decision**, sized for one maintainer and a few testers:
  - **Nightly**: every new commit on `main`, required checks green,
    signed and notarized. No gate.
  - **Beta**: the build was published 3+ days ago and has been used as AI
    Juggler Nightly for real work, and no issue labelled `blocker` is
    open; notes written; the accessibility gate passes.
  - **Stable**: 14+ days on beta (from the annotated `beta/vX.Y.Z` tag's
    date), no open `blocker`, the beta gate signed, a CHANGELOG section;
    and the maintainer chooses to release it.
  - **Blocker**: data loss, an Instance on the wrong account, stopping a
    process AI Juggler doesn't own, a crash at launch, a broken update or
    rollback.
- **Enforced** by `scripts/promotion_rules.py` (self-tested in CI) from
  `promote.yml`. A rule is overridden only with a written reason, kept in
  the channel tag.
- **Later**: a crash-free-sessions threshold for stable once crash reports
  have a service, and a short fixes-only freeze before a stable release
  (one version stream means a beta fix otherwise brings other changes).
- **Applied first to 0.0.4**: held until it meets the beta requirements.

### D-25 The UX vision: progressive by design; "session" on screen; first run in three steps — SETTLED (2026-10-19)

- **Ruling** (maintainer, after first real use of 0.0.4): the app showed
  everything it knew, everywhere, at once — nine top-level tabs, seven
  filters and a full status legend over an empty list, internal codes on
  screen, a first run that opened on a CLI check. It must be "smart
  progressive": what a multi-subscription vibe-coder needs at a glance,
  depth one step away or on request. Reviewed and approved as the canvas
  "AI Juggler UX vision" (vision + first run, OS access, Home, a session up
  close, an account hitting its limit).
- **Principles**: lead with "what needs me?"; runway always in view; calm
  until it matters (colour only for what needs you, every state also a
  shape); depth on request (summaries, then Details, then ⋯/Settings);
  plain words; an empty screen means "start here".
- **Structure**: top level Home, Accounts, History, Settings; Cleanup shows
  when there is something to clean, Fleet once a second Mac is paired;
  density moves to Settings › Appearance. One rail of projects with their
  sessions, those that need you first; search plus Needs you · Running ·
  All, the rest behind a Filter button; the legend behind "?". Home: Needs
  you, Runway, Running, Pick up where you left off. Runway lives on Home and
  in Accounts; a row shows quota only when it's low.
- **Wording**: the screen says **session** for a conversation running on an
  Account in a project (what the UI called an Instance). The domain model
  and code keep `Instance`/`Session`; only user-facing text changes.
- **First run**, when no Account exists: (1) what's already on this Mac —
  the CLI's config and Claude desktop profiles, grouped, preselected, one
  button to bring them in (copies; originals untouched); (2) sign in where
  needed; (3) OS access, then Home. The pack-secrets question moves from
  first run to the first switch, which already asks while no choice exists
  (D-14 unchanged: nothing is built before a choice).
- **OS access is per platform**: the backend lists what this OS needs, each
  with one line of why, live status, an Allow that opens the system prompt
  and a link to the settings pane (macOS: Accessibility, Screen Recording;
  Local Network only when Fleet is turned on). Another OS lists its own, or
  none — then the step is skipped. Anything skipped is asked for in
  context, once.
- **Consequences**: the existing wizard stays for Accounts › Add account
  until it is simplified in a later stage (done 2026-10-20, AJ-E1022: a
  fresh account and a folder import are first run's own steps, the
  wizard is retired; AJ-E1023: Add account asks which tool only when
  more than one is installed); screens are rebuilt in stages
  (first run; Home and rail; the session view, Accounts and the rest), each
  walked as a real journey before it merges.
- **Amendment (2026-10-20, E14 / AJ-E1402): AI tools AI Juggler didn't
  start get no rail rows.** The Claude desktop app's windows and `claude`
  in a terminal are measured read-only and shown in Home's **This Mac**
  (the heaviest three, plus "Also counted: N Claude tools not started by
  AI Juggler"). They count in its totals and are always labelled as not
  ours. They do not appear in the rail. Evidence: the reference Mac runs
  six Claude desktop windows. In a walk of the rail with a quiet "Also
  running" group of six such rows
  ([mock-up](../qa/evidence/AJ-E1402/req5-rail-mockup-1f0b5cd1.jpg)), they
  pushed the user's own sessions down. They looked openable but had nothing
  to open: no project, no conversation, no actions (D-08). And they repeated
  what This Mac already says. The rail stays "your projects and their
  sessions". A resource figure shows on a rail row only while that session
  is heavy (the same rule as low quota).
- **Amendment — Triage and Overview fold into Home** (2026-10-20,
  AJ-E1021; the maintainer's brief for stage 4 asked for the evidence):
  - *Evidence.* Triage's four groups were Home's Needs you and Running plus
    two extensions: parked sessions (which Home showed as "hit its limit",
    a decision already made) and ended-but-not-reviewed (Home's "Pick up
    where you left off", without the reviewed step). Its sort duplicated
    the rail's Sort (needs, quota risk, project). Overview repeated
    Running and Runway as cards, and two of its four cards were fixed
    values ("Machine OK", "Fleet 1"). Both sat under More, outside D-25's
    structure.
  - *Decision.* Home gains "Parked until a reset" (Resume once the reset is
    reached), lists what ended this week and isn't reviewed (three, then
    Show all; Mark reviewed takes one off), and keeps AJ-E1005's keyboard
    (J/K or arrows, Enter, A), named in one quiet line. The ledger-degraded
    banner moves to Home. Triage and Overview leave More (Usage, Fleet,
    Cleanup remain). AJ-E1005's requirements stand, met on Home; sorting
    is the rail's.
- **Amendment — no internal codes on screen; unbuilt actions wait**
  (2026-10-20, AJ-E1025):
  - *Evidence.* The session, account, project and tool views showed task
    and decision codes as text ("lands with AJ-E606+", "Switch Graph =
    E2", "audit-logged (D-08)"), raw ids ("claude-code", "quota_blocked"),
    "version: No data" chips, and a dozen disabled actions whose only
    reason was the task that would build them.
  - *Decision.* What the user reads names things in plain words; codes
    stay in comments, docs and tests (`scripts/terminology_lint.sh` fails
    on a task or decision code in JSX text, a visible attribute or a
    string). An action that isn't built yet isn't shown; one that exists
    but doesn't apply now stays disabled with its reason (AJ-E407 Req.4,
    amended). A row with nothing to show waits until it has data. The
    tool view (what's installed, its accounts' quota) opens from the tool
    chip in a session's or an account's header.

### D-26 Open at login — a second audited unsafe shim — SETTLED (2026-10-22)

- Decision: juggler-login-item: SMAppService via objc2-service-management
  0.3.2, approved by the maintainer 2026-10-22 (AJ-E1509 Req.4, "Open AI
  Juggler at login", opt-in). macOS's own login items are `SMAppService`,
  and every call to it is `unsafe` in its Rust bindings. As with D-17, the
  carve-out is a NEW, isolated crate (`crates/juggler-login-item`) that
  does NOT inherit the workspace lint baseline; the workspace law
  `unsafe_code = "forbid"` stays absolute for every other crate. It holds
  exactly four `unsafe` calls (the main app's service, its status,
  register, unregister), each with a SAFETY note, under
  `#![deny(unsafe_op_in_unsafe_fn)]`, behind a safe API (`status`, `set`).
  Off macOS it is a safe stub that reads "unavailable".
- Rejected: a `~/Library/LaunchAgents` plist (writes outside AI Juggler's
  data folder, and macOS lists it under "Allow in the background", not as
  a login item).
- Consequences: off by default; Settings › Sessions and the welcome's Make
  it smoother card offer it; the system keeps the choice (AI Juggler
  reads it back). Registering works only in the installed, signed app; a
  development build reads "unavailable", so the live check is an installed
  build's.

### D-27 The compressor's pages — a third audited unsafe shim — SETTLED (2026-10-23)

- Decision: juggler-vm-stats: one `host_statistics64(HOST_VM_INFO64)`
  read, approved by the maintainer 2026-10-23 ("Approved: D-27, an
  audited one-call crate for host_statistics64 so This Mac counts
  compressor pages like vm_stat"). This Mac's memory used is app memory +
  wired + compressed, as Activity Monitor and `vm_stat` count it; the
  compressed part is the pages the compressor occupies
  (`vm_statistics64.compressor_page_count`), which no sysctl gives. As
  with D-17 and D-26, the carve-out is a NEW, isolated crate
  (`crates/juggler-vm-stats`) that does NOT inherit the workspace lint
  baseline; `unsafe_code = "forbid"` stays absolute for every other
  crate. It holds exactly three `unsafe` blocks (the host port, asked
  for once and kept; zeroing the statistics struct; the call), each with
  a SAFETY note, under `#![deny(unsafe_op_in_unsafe_fn)]`, behind one
  safe function (`compressor_pages`). Off macOS it is a safe stub that
  reads nothing. Its test holds it to `vm_stat`'s "Pages occupied by
  compressor".
- Rejected: `vm.compressor_bytes_used` (the compressed data inside those
  pages: memory used read about 3 % of physical memory low on the
  maintainer's Mac, the 0.1 real-data pass 3.4); spawning `/usr/bin/vm_stat`
  on every reading (a process every few seconds for as long as the app
  runs).
- Consequences: This Mac's memory used now matches `vm_stat` and
  Activity Monitor. If the kernel refuses the read, there is no memory
  reading at all (never a sum with another measure under the same name).

## Unresolved (in discussion order)

- None. D-01..D-15 were marked Final on maintainer instruction
  (2026-10-13); D-16 and D-17 were settled by maintainer instruction
  on 2026-10-13, D-18 on 2026-10-17, D-19 on 2026-10-18, D-20..D-25 on 2026-10-19, D-26 on 2026-10-22 and D-27 on 2026-10-23; all follow the same law.
