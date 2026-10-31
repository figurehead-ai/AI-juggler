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
- *Amended 2026-10-27 (the maintainer, on the switch dialog: "Isn't
  the whole point of transferring to another account so that it is
  not the usual account? isn't this message pointless?"):* the target
  not being the project's usual account is no risk. It no longer
  opens the wizard and is never a warning; "usual account" stays only
  as a positive hint in the target list ("Used most here"). A
  compacted source is no risk either: a same-tool switch copies the
  whole session file and the target resumes it as the source tool
  would, and a replay reads every turn, newest first while they fit.
  The compaction reason, the compaction cap on fidelity and the
  "(compacted)" label are gone. The wizard's warnings are real,
  actionable risks only: little room left on the target, something
  that stays behind, a conflict, or a fresh start instead of a resume.

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
    signed and notarized. No gate. *Amended 2026-10-27:* a scheduled
    nightly builds only when something that ships changed since the
    newest release tag (`scripts/release_ships.py`), so a docs- or
    tests-only day takes no version number (v0.1.5 and v0.1.7 were
    identical rebuilds); a run started by hand still builds.
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

### D-28 What agents may do: access levels per pool, rules you can revoke — SETTLED (2026-10-24)

- Decision: an agent calling AI Juggler's tools from inside its own
  session (a status, a message, a shared doc, a task row) is recorded as
  the agent's action, not one the app executes. What it may do is set by
  its pool's **access level**, which only the user sets, in the app:
  **Read** (the roster, cards and tasks); **Work together** (also its
  status, messages, shared docs, and creating, claiming and updating
  tasks); **Full** (also delivering into other members' sessions, D-29);
  **Custom** (per action). A project's default pool starts at **Work
  together**. An action above the level becomes an **ask** in Needs you
  (grouped, never a pop-up): Allow once, Always allow in this pool, or
  Deny. On the **second** identical approval (same action, same pool),
  the ask offers to remember it as a rule. Every level and rule is on
  one page (Settings › Agent access, and each pool's page), revocable at
  any time, effective on the next call; every agent action is logged.
  The maintainer, 2026-10-24: "what we don't want to do, is get into a
  mess where user is constantly being waited on to approve things … if
  it sees user constantly being asked to approve certain types it can
  offer an option to create a rule for it … all of these could be
  revoked by user in our system at any time".
- Hard lines no level or rule lifts: agents never change pools, levels
  or rules; agents never start a session or choose an account (D-31);
  agents never get credentials; AI Juggler never answers a tool's own
  permission prompts by itself (amended by D-40, 2026-10-25: the user's
  own click in AI Juggler is carried to the tool, and "don't ask again"
  is the tool's own rule, picked from the options the tool offers, so
  it is visible and revocable in the tool too).
- Rejected: every agent write approved one by one (the approval spam
  this ruling exists to avoid); reads only (messaging and tasks would
  wait).
- Consequences: one decision point, `allowed(pool, action)`, answers
  Allow, Ask or Deny for every agent action (AJ-E1702); the bridge
  (AJ-E1704), messaging (E18) and tasks (E19) only call it.

### D-29 Delivering into a session without a click — SETTLED (2026-10-24)

- Decision: AI Juggler may deliver a message into an idle session, so it
  starts a turn without the user, only where the user allowed it: a pool
  at **Full**, or a rule. Every delivery lands only between turns (never
  mid-turn, over the user's typing, or at a tool's permission prompt);
  is labelled with its sender and that it can't approve anything or
  change settings; shows in the session as started by that message and
  goes in the audit log; uses the vendor's own route where one exists
  (Claude Code's messages between sessions), else is typed as the next
  prompt, through the same checks. Caps apply on every level: a **hop
  limit of 5** by default (one agent waking another is a hop; a person's
  prompt, click or reply resets the count; at the limit the chain pauses
  and asks), changeable per pool; a rate limit; duplicates dropped; a
  stop-all switch per pool.
- Rejected: never waking (agents couldn't hand work over while the user
  is away); waking in every pool (one runaway pool could spend every
  account).
- Consequences: an amendment to SPEC §1 and ADR-0003's "sessions act
  only when you act": the user's level or rule is the permission.

### D-30 Adding AI Juggler's tools to sessions, pre-allowed — SETTLED (2026-10-24)

- Decision: AI Juggler adds its agent tools to the sessions it starts,
  through each tool's launch options (`--mcp-config` with a per-session
  file in AI Juggler's own folder, never `--strict-mcp-config`; Codex's
  `-c mcp_servers.*`), never by editing the user's config. It asks once,
  the first time pools are used ("Let agents in your projects see each
  other and work together?"); after that it's on, and can be switched
  off per project or account. AI Juggler's own tools are **pre-allowed**
  at launch (`--allowedTools` for them; Codex's approval setting for
  them), so no session prompts for them: the pool level (D-28) is the
  real check. Every other tool's prompts stay as the vendor shows them.
- Rejected: off until enabled per project; on with no question;
  pre-allowing read-only tools only or nothing (each new session would
  greet the user with AI Juggler's prompts).
- Consequences: sessions started outside AI Juggler are pool members but
  can't call the tools until continued in AI Juggler.

### D-31 Agents never start sessions; nothing picks an account by usage — SETTLED (2026-10-24)

- Decision: an agent may **prepare** a launch (project, task, a draft
  prompt), shown in Needs you; the user picks the account and starts it.
  Nothing ranks or chooses accounts by remaining usage, for agents or
  tasks; task assignment never reads quota.
- Rejected: launches by rule on a fixed account (more cost running
  unwatched; may be revisited after pools are in use); agents choosing
  any account (rotating accounts to get past usage limits is what the
  vendors' terms most clearly forbid; ADR-0003).
- Consequences: a guardrail test keeps pools, assignment and the bridge
  free of quota types.

### D-32 Pools across your Macs, opt-in per pool, in 0.2 — SETTLED (2026-10-24)

- Decision: a pool may include sessions on the user's paired Macs,
  switched on per pool and off by default. Then its message, shared-doc
  and task content travels between those Macs over juggler-link,
  directly and encrypted, with no server. The pool's page says so when
  it's switched on.
- Rejected: later (the maintainer chose to build it in 0.2); never.
- Consequences: the Privacy page's Fleet line ("It never gets your
  sign-ins, files or prompts") and its link contract tests are rewritten
  to say exactly when pool content travels, and only for pools switched
  on. The two-Mac Fleet check (AJ-E710, the maintainer's) becomes part of
  0.2's verification.

### D-33 The default pool holds a project's sessions; agents see account labels — SETTLED (2026-10-24)

- Decision: a project's default pool holds all its sessions, on any
  account. Agents' context cards may show a member's **account label**
  (the name the user gave it), never an email, plan or other account
  detail.
- Rejected: separate default pools per identity (Work and Personal)
  within a project; one pool across all projects; full account details
  on cards.
- Consequences: a user who keeps Work and Personal apart does it with
  their own pools or by project. A test keeps emails and plans off cards.

### D-34 Keys for accounts through another service live in the Keychain — SETTLED (2026-10-24)

- Decision: an account that runs Claude Code or Codex through another
  service (OpenCode Go, Kimi: their published Claude- or
  Codex-compatible address) keeps its key in the macOS Keychain, held
  by AI Juggler. The user pastes it once in Add account; AI Juggler
  hands it only to that account's sessions, as the environment variable
  the tool expects, and never writes it to a file, a log, the search
  index or a backup, never shows it again, and sends it nowhere else.
  Removing the account removes the key. Kimi usage may be read from
  Kimi's own local server (`kimi web`) only when the user switches it
  on for that account and only while that server runs: AI Juggler reads
  its token into memory, calls only its usage address, and keeps
  nothing. The daily check for newer tool versions (npm's registry)
  starts on. The maintainer chose each of these on 2026-10-24, with the
  options set out in full. (Brand marks for OpenCode, Kimi and Grok
  were not ruled; they stay letter marks, as every unapproved mark does.)
- Amends: E16's law and AJ-E1601 Req.2 ("AI Juggler never reads,
  copies or stores" the key), and the Privacy page's "no-proxy" line:
  still no proxy, and a provider address is set only for an account
  added "through another service", to the address that service
  publishes. The Privacy page names the Keychain key and the opt-in
  Kimi usage read.
- Rejected: the key in the account's settings file (plain text; Codex
  takes only an environment variable); in the user's shell profile
  (plain text, every terminal); not building it.
- Consequences: AJ-E1601 builds it, with guardrail tests (no key in
  argv, files, logs, the index or backups); the "never read the
  environment" rule (D-37) gains a test that AI Juggler's own key reaches
  only the launch it starts.

### D-35 Profiles are not accounts — SETTLED (2026-10-24)

- Decision: a **profile** is the setup an app runs on: a Claude Code
  folder (`CLAUDE_CONFIG_DIR`), a Claude app profile (its data folder),
  a `CODEX_HOME`. An **account** is the subscription a profile is
  signed in to. One account can have several profiles, and the same
  login in two profiles is normal. The screens show profiles where
  sessions run (rows, places, badges) and accounts where limits and
  money live (runway, the Accounts page). Launch settings sit under a
  profile. The maintainer, 2026-10-24: "We are clumping accounts with
  profiles, which is not the same. Profile / app is what each native
  app runs on, you can have multiple profiles for the same
  subscription account."
- Amends: D-02 and D-10 ("profile" meant launch settings and was never
  a rail noun), ADR-0008 §5, and E3's "same login twice" warning. The
  stored entity stays one per folder in 0.1.x; accounts are worked out
  from each profile's sign-in (account and organisation) when read.
- Rejected: keeping "account" for a folder (the maintainer's
  complaint).
- Consequences: AJ-E2109; the terminology lint learns the new meaning.
  (D-28 to D-34 are above.)

### D-36 Desktop first: the rail shows what is open, in every app — SETTLED (2026-10-24)

- Decision: AI Juggler's main view is what is really open on the Mac,
  grouped by **place** (the app and profile a session runs in), with
  what needs the user first and one click to get there. Sessions other
  apps run are shown when they are open; ended, inactive, background
  and other sessions stay one click away through View and the rail's
  "Also:" line, never shown by default and never removed. The
  maintainer, 2026-10-24: "the whole point of our app is to show
  projects, sessions and actual Native Apps themselves and get to them
  … users … just want to get quick assessment where is what", and
  "we should have a way to show or filter other sessions … just not
  enabled by default".
- Amends: D-25 (the rail as projects with their sessions; the E1402
  amendment that gave tools AI Juggler didn't start no rail rows), and
  ADR-0007 decisions 0 and 2 (projects as the anchor): projects become
  one lens, Place is the default.
- Consequences: E21; the approved "desktop first" canvas.

### D-37 Knowing what is open without reading secrets — SETTLED (2026-10-24)

- Decision: what is open comes from the apps' own per-session files and
  each process's parent chain and arguments, never from a process's
  environment (the Claude app's sessions carry a sign-in token there),
  key files or token caches. Status words are read; summary text is
  not. Going to a session activates the app by its process; exact
  windows and terminal tabs use Accessibility and Automation only when
  the user turned them on in Settings, and a click never raises a
  permission prompt. Every result says what really happened.
- Rejected: reading environments to find a session's profile (the 0.1
  code did; removed in AJ-E2101); opening by bundle id (it brought the
  wrong Claude profile forward); scraping the Claude app's chat cache
  (undocumented, and its terms forbid automated access to chats).
- Consequences: AJ-E2101, AJ-E2102; guardrail tests; the Privacy page
  names the new reads.

### D-38 Apps' icons come from the apps installed on the Mac — SETTLED (2026-10-25)

- Decision: wherever AI Juggler names an app a session runs in (the
  Claude app, ChatGPT, Terminal, iTerm, an editor, AI Juggler itself),
  it shows that app's own icon, read at run time from the app as it is
  installed on the user's Mac. The maintainer, 2026-10-25: "Yes: show
  each app's icon as installed on my Mac, read at run time; nothing
  shipped in the repo."
  - The icon comes from the app's bundle on this Mac, through macOS
    (`NSWorkspace`), only for an app AI Juggler already knows: one a
    session is open in, or one in the host registry. Only a folder
    ending in `.app` is read, never a link, and nothing outside it.
  - Nothing is committed to the repository: no app's icon, no copy of
    one in a test or a preview (those draw a made-up test image).
  - A small copy is kept in AI Juggler's own folder (`icons/`), redone
    when the app's bundle changes. Nothing is sent anywhere.
  - When the app isn't installed or its icon can't be read, a clean
    one- or two-letter mark stands in, never cut off.
  - The profile shows as its own badge beside the icon, never over it;
    one app's profiles share its icon.
- Amends: nothing. The marks of tools and companies (`ToolMark`,
  AJ-E1526: Claude Code, Codex, Anthropic, OpenAI) stay under their old
  rule: drawn only with their owner's permission, otherwise AI
  Juggler's own letter mark.
- Rejected: bundling the apps' icons (the owners' artwork in a public
  repo); fetching them from the network.
- Consequences: the `app_icons` feature module, `AppIcon` and
  `ProfileBadge` in the UI, a Privacy line; ships in 0.1.6 (planned as 0.1.5; that number went to the 2026-10-26 nightly).

### D-39 Fresh Claude limits when the Claude app's meter is old — SETTLED (2026-10-25)

**Ruling.** The maintainer, 2026-10-25: "Yes to D-39 option A: run Claude
Code's own /usage check (claude -p /usage, hooks off) for profiles whose
reading is old, at most every 15 minutes while in use. Reading the Claude
app's plan-usage-history.json is fine too." It is on by default for every
profile; each profile's page turns it off. Proven live the same day on a
real app-signed-in profile (AJ-E1704 lab, 2026-10-25): no model call, no
cost, `cachedUsageUtilization` refreshed. Correction from that lab: Claude
Code also leaves an empty `projects/<folder>/memory/` folder of its own.

**The problem.** The maintainer reported it three times: every Claude
account read "Not measured yet" for both windows. Claude Code reports its
limits to a status line, and only the terminal UI runs a status line. The
Claude app runs Claude Code headless (`entrypoint: "claude-desktop"`), so
the status line never runs there. On the maintainer's Mac, every
transcript of the last 7 days came from the Claude app (813) or the SDK
(82), none from a terminal. The AJ-E1530 capture could never have measured
these accounts, even with consent.

**What already ships, with no permission and no setting changed** (this
branch). All of it is read-only, on the Mac, and named on the Privacy page:

1. **The Claude app's own meter.** Each app instance (each
   `--user-data-dir`) keeps `plan-usage-history.json`: 5-hour and weekly
   percent used, about every 15 minutes while the app is open, by
   organisation. It is matched to a profile through the instance's
   `claude-code-sessions/<account>/<org>/` folder names. On the
   maintainer's Mac every instance maps to exactly one profile. This is
   the main source. It has no reset times.
2. **Limit hits in transcripts.** Claude Code writes `error: "rate_limit"`
   with `quotaLimits {rateLimitType, resetsAt}`. That gives "Hit its limit
   at 14:02 · resets 19:00", with the vendor's own reset time.
3. **Claude Code's last `/usage`.** `.claude.json` keeps
   `cachedUsageUtilization`: both windows with their resets. It is written
   only when `/usage` runs, or when an app asks Claude Code for usage.

The question is what to do when the app isn't open on a profile, so its
meter goes old. A 5-hour reading older than 5 hours is shown only as
history. Each option below says what the user sees, what macOS asks, what
it costs, what leaves the Mac, and how to undo it.

**Option A: ask Claude Code itself (recommended).** AI Juggler runs
`claude -p /usage --no-session-persistence --strict-mcp-config --settings
'{"disableAllHooks":true}'` with the profile's `CLAUDE_CONFIG_DIR`, from an
empty folder of AI Juggler's own.
- What the user sees: per profile, on its page, "Ask Claude Code for its
  limits", off. Turning it on opens one sheet that says what runs. While
  on, it runs at most every 15 minutes, only when the profile was used in
  the last hour and its newest reading is older than 15 minutes. There is
  also "Measure now".
- macOS: nothing. Claude Code reads its own Keychain item, the same as
  every time it starts. AI Juggler never touches the Keychain or the
  token.
- Cost: no quota. `/usage` is a local command (`supportsNonInteractive`
  in Claude Code 2.1.302's command table), so no message goes to Claude.
  It costs one short process per run (about a second), no transcript, no
  MCP servers and no hooks.
- What leaves the Mac: AI Juggler sends nothing. Claude Code makes the
  same request to Anthropic that `/usage` makes when the user types it.
  The trust page's promise ("It makes no requests to … any AI provider.
  Your tools talk to their providers themselves") still holds.
- What changes: Claude Code updates `cachedUsageUtilization` in that
  profile's `.claude.json`. That is its own file, written by Claude Code
  the same way it writes it when the user types `/usage`. AI Juggler
  writes nothing there.
- Undo: Turn off in the same place. Nothing runs after that, and nothing
  is left behind.
- Not yet verified live: the maintainer's rule forbids running `/usage`
  on the real accounts outside the switching lab. The flags and the
  command table were checked in the 2.1.302 binary. A stand-in `claude`
  proves the exact command line and folder.
- Built behind the setting `limits_probe_allowed`, which is off, so the
  UI offers nothing until it is ruled. Tests: `claude_limits_records`
  (`asking_claude_code_is_refused_until_it_is_ruled_on`,
  `once_ruled_on_the_probe_runs_claude_codes_usage_in_the_profile_and_keeps_no_output`)
  and vitest `claude-limits-states`. Shipping it means setting that
  default to on.

**Option B: AI Juggler calls Anthropic's usage endpoint itself.**
- What the user sees: the same numbers, a little faster.
- macOS: a Keychain prompt per profile: "AI Juggler wants to use your
  confidential information stored in 'Claude Code-credentials…'". "Always
  Allow" would give AI Juggler lasting access to each account's sign-in
  token.
- Cost: no quota, but AI Juggler would hold an OAuth token in memory. The
  endpoint is undocumented, and the token's refresh can't be handled
  without writing to the vendor's item.
- What leaves the Mac: AI Juggler itself would make requests to
  Anthropic. That breaks two Privacy-page lines: "never reads … the tools'
  Keychain items" and "makes no requests to … any AI provider".
- Undo: revoke in Keychain Access, per item.
- Not recommended. Not built.

**Option C: the status-line capture (AJ-E1530, already built, opt-in).**
- It covers only `claude` run in a terminal. Claude's docs say the status
  line gets `rate_limits` "only for claude.ai Pro and Max subscribers",
  not Team seats.
- It changes `settings.json` with consent. It is refused for profiles
  without a `settings.json`, which three of the maintainer's profiles
  lack.
- Kept as it is, re-worded as "Also measure terminal sessions…". It is
  never offered as the fix for app sessions.

**Option D: nothing more.** The app's meter, limit hits and any `/usage`
the user runs are shown with their age. A profile whose app isn't open
shows "Last reading 13 h ago: 40% used; that window has reset since".

- Recommendation: **A**. It is the only fresh source that needs no macOS
  prompt, never handles a token, sends nothing from AI Juggler and costs
  no quota. It is off per profile until its owner turns it on.
- Also needs a ruling: whether reading the Claude app's
  `plan-usage-history.json` is acceptable. It is built and shipping on
  this branch: no chat content, no token, read-only, named on the Privacy
  page. It is not the chat cache D-37 rejected.

### D-40 How AI Juggler runs the sessions it starts: a runtime per tool — SETTLED (2026-10-25)

- Decision: option A of the research (`docs/research/session-runtime.md`,
  ADR-0022). The maintainer, 2026-10-25: "Yes to D-40 option A: Codex
  through app-server by default, Claude terminal + hooks by default with
  headless as a Beta choice, no bundled SDK. Also yes to amending D-28 to
  'never by itself'."
  - Codex: each session AI Juggler starts is driven through the user's own
    `codex app-server` (one per session, a socket in AI Juggler's data
    folder, owned in the spawn ledger). Approvals and questions are
    answered in AI Juggler; state and the 5-hour and weekly limits come
    from the protocol; "Open in terminal" attaches the real TUI with
    `codex --remote`. The PTY with `--no-daemon` stays the fallback. The
    protocol version is pinned and its schema regenerated each release.
  - Claude: the real terminal stays the default, with exact state and
    limits from Claude Code's own hooks and status line through a
    per-launch `--settings` file in AI Juggler's folder. "Run without a
    terminal: answer from AI Juggler" is a per-session Beta choice:
    AI Juggler speaks Claude Code's stream-json protocol from Rust to the
    user's own `claude`, and hands back to the terminal at a turn boundary
    with `--resume`. It becomes the default only after four gates
    (ADR-0022 decision 4): Anthropic's published terms, rechecked each
    release, still allow it; `-p` keeping subscription sign-in once
    `--bare` is its default; a month of lab use with no billing change;
    the conversation view passing a hand walk. (Amended by the maintainer,
    2026-10-25: no written approval is sought. The Beta never uses the
    Agent SDK; it runs the user's own installed `claude -p`, which
    Anthropic's Help Center of 2026-10-07 says may use subscription
    limits, and AI Juggler never handles a sign-in.)
  - No vendor SDK is bundled. AI Juggler speaks each tool's protocol to
    the user's own installed binary. The terminal stays a first-class
    runtime for every tool, and the only one for tools without a protocol.
- Quoted terms (research §2): Anthropic does not "prevent an end user from
  signing in to the unmodified Claude Code binary with their own Claude
  subscription", and does not allow "third party developers to offer
  claude.ai login or rate limits for their products, including agents
  built on the Claude Agent SDK" unless approved. OpenAI lets "a local or
  open-source application" use Codex app-server with ChatGPT sign-in;
  never for commercial or hosted services.
- Amends: D-28's hard line (above), now "never answers by itself".
- Rejected: bundling the Agent SDK or the Codex SDK (policy, about
  450–700 MB, a second Claude Code copy, and the Codex TS SDK can't answer
  approvals); terminal only (turns down the protocol Codex's own TUI
  uses); headless by default for both now (loses Claude's TUI before the
  policy questions settle).
- Consequences: ADR-0022 Accepted; a runtime trait in `juggler-adapter`
  (Terminal or Driven, per launch), the conformance kit testing both
  runtimes; a live spike on spare accounts first (the maintainer's step,
  as AJ-E1701); `docs/research/codex.md` §9 gets an addendum (app-server's
  rate limits are documented); the Privacy and terms pages name the
  runtime.

### D-41 Processes of sessions other apps run: remembered, and stoppable by the user — SETTLED (2026-10-25)

- Decision: the maintainer, 2026-10-25: "Yes to both: (a) a local 7-day
  names-only record of processes in sessions other apps run, and (b)
  user-chosen, verified stop of their leftovers after the session ended,
  never automatic, never the app or the session's own process. Build the
  Processes view in 0.2."
  - (a) Sightings. For a session another app runs (the Claude app, the
    Codex app, a terminal `claude`), AI Juggler keeps a local record of
    its processes: pid, start time, program name, the session it belongs
    to, first and last seen, and a hash of its command line. Never the
    environment, never the command line itself, nothing sent anywhere.
    Kept 7 days. It lets a helper that outlives its session show as "left
    behind by <session>".
  - (b) User-chosen stop. A leftover of such a session may be stopped only
    by the user's own click, only after the session's own process has
    ended, through the same identity check (against the sighting: pid,
    start time, program, boot), the same verify sheet and the same audit
    (`actor = user:observed`). Never automatically, at any tier; never the
    app itself or a session's own process.
- Amends: D-08 (unowned processes untouchable): now "never by AI Juggler
  on its own; the user may stop a verified leftover of another app's
  ended session". AJ-E1402 Req.3 ("the ledger is never written for
  them"): the ledger still isn't; sightings are their own record.
- Why: on the maintainer's Mac every live session runs in the Claude app
  or ChatGPT, with ~110 child processes and 16 orphans up to 3 days old
  that AI Juggler could neither show per session nor act on (the
  processes gap analysis, 2026-10-25).
- Consequences: the 0.2 Processes view (app → session → process →
  subprocess, one model for Cleanup, a session's Processes tab and This
  Mac); a sightings store with guardrail tests (no environment, no command
  line, 7-day expiry); the collateral suite extended to observed
  processes; Privacy page lines.

## Unresolved (in discussion order)

- D-43 points 4 (a task's brief as a session's first prompt) and 6 (the
  Claude Code plugin outside AI Juggler's launches): approved as built,
  to be revisited with the maintainer after the messaging and tasks labs.
  The plugin stays unpublished until then.
- D-01..D-15 were marked Final on maintainer instruction
  (2026-10-13); D-16 and D-17 were settled by maintainer instruction
  on 2026-10-13, D-18 on 2026-10-17, D-19 on 2026-10-18, D-20..D-25 on 2026-10-19, D-26 on 2026-10-22, D-27 on 2026-10-23, D-28..D-37 on 2026-10-24, D-38..D-41 on 2026-10-25, D-42..D-43 on 2026-10-26, D-44..D-49 on 2026-10-27, D-50..D-51 on 2026-10-28 and D-52..D-57 on 2026-10-29. All follow the same law.

### D-42 Messages between agents: secrets, reading, where they show, and four smaller points — SETTLED (2026-10-26)

**Ruling.** The maintainer, 2026-10-26: "Approve D-42 and D-43 as built;
revisit D-43 point 4 (task brief as first prompt) and point 6 (plugin)
after the labs."

Context: E18 (`docs/tasks/E18-agent-messaging.md`) built on D-28 and D-29.
Six choices weren't ruled by them. Each is built as recommended here and
covered by tests, so changing one is a small, local change.

1. **Secrets in messages and shared docs follow the packs' choice (D-14).**
   - What the user sees: a message or doc with a key in it shows "1 secret
     withheld, as you chose for packs" and `•••` in place of the key; with
     **Keep** chosen in Settings › Privacy, the key stays (on this Mac).
   - Options: (a) always strip, whatever the choice; (b) the packs' choice,
     with Ask each time and "not chosen yet" stripping (no one is there to
     ask when an agent writes); (c) a separate setting for messages.
   - Cost and risk: (a) overrides the user's choice, against D-14's "always
     the user's choice"; (c) is one more setting to explain. (b) means a user
     who chose Keep lets agents pass keys to each other, on this Mac only.
   - Recommendation: **(b)** (built).
2. **Reading its own messages is part of Read.** A new catalog action,
   `read_messages` ("Read their messages and the shared docs"), in Read and
   every level above.
   - What the user sees: one more line in a Custom level's list; a pool at
     Read can still hear from the user, and answers only with the user's OK.
   - Risk: none new (reading never sends). Alternative: put it under Work
     together, so a pool at Read can't even read what the user wrote to it.
   - Recommendation: **in Read** (built).
3. **No separate Inbox screen.** E18's "Inbox pane" is each pool's
   **Messages** tab, each session's **Messages** tab (shown once it has
   any) and Home's **Needs you** for what waits on the user, following
   E21's desktop-first ruling (no separate pools section).
   - Cost: no single list of every pool's threads; a user with many pools
     goes pool by pool, while Needs you gathers what needs them.
   - Recommendation: **keep it so for 0.2**; add an all-pools list only if
     the lab or the maintainer's use asks for it.
4. **Claude Code's own messages between sessions are not used in v1.**
   D-29 says to use the vendor's route where one exists. Claude Code's
   cross-session messages need `crossSessionInbound`, which AI Juggler must
   not change without the user, and reach only Claude sessions. v1 delivers
   through the driven runtime (`send_turn`) for every tool, and a Claude
   session in a terminal reads its inbox when told by its hooks.
   - Recommendation: **revisit after the messaging lab**
     (`docs/qa/messaging-lab.md`), when the route's behaviour is seen live.
5. **The turn note also comes after each tool call** (`PostToolUse`), not
   only when a turn starts, so a busy agent hears about a message during a
   long turn. It is said once per change and never carries a message's
   words.
   - Risk: a little more context per change; an agent may stop to read.
   - Recommendation: **keep** (built); drop `PostToolUse` if the lab finds
     it distracting.
6. **Smaller points, built as stated:** a thread takes at most 6 agent
   messages in a row (E18), and the pool's hop limit (5 by default, D-29)
   counts the hops already taken before a delivery, so 5 automatic wakes
   pass and the next pauses and asks; the user's compose has **Deliver to
   their sessions** on by default (the user is the one writing); linking
   two pools is the user's, on a pool's Agent access tab, both ways at
   once; E18's guardrail "no message reaches a session's input without a UI
   command" is read as D-29 ruled it: without the user's click, a pool at
   Full, or the user's rule.
   - Note: E17's **Include my paired Macs** says "Message and task content
     will travel to your other Macs", but AJ-E1705 (pools across Macs)
     isn't built, so nothing travels yet; a guardrail test proves messages
     never reach the Fleet link. Recommendation: say "will travel once pools
     across Macs ship" until AJ-E1705 lands.

### D-43 Tasks shared by pools: held changes, honest states, a session for a task, issue titles, and the plugin — SETTLED (2026-10-26; points 4 and 6 revisited after the labs)

**Ruling.** The maintainer, 2026-10-26: "Approve D-42 and D-43 as built;
revisit D-43 point 4 (task brief as first prompt) and point 6 (plugin)
after the labs."

Context: E19 (`docs/tasks/E19-agent-tasks.md`) built on D-28 and D-31.
Seven choices weren't ruled by them. Each is built as recommended here and
covered by tests, so changing one is a small, local change.

1. **An agent's task change above its pool's level is held, not refused.**
   - What the user sees: in Needs you, "«Planner» wants to give aj-9e41 to
     Reviewer" with Allow once, Always allow in this pool (from the second
     time) and Deny, the same card as a held message (E18). A yes applies
     it as the agent's own, logged on the task; a no logs "you said no".
     The agent is told it's held and goes on with other work.
   - Options: (a) hold it (as E18 holds a message); (b) refuse it, and the
     agent tells the user in words.
   - Cost and risk: (a) one more kind of card in Needs you; giving a task to
     someone else (`assign_task`, in no named level) always asks until the
     user makes the rule, which is D-28's design. (b) loses the agent's
     intent and makes the user redo it by hand.
   - Recommendation: **(a)** (built).
2. **Honest states.** A task is claimable only when its status is
   **Ready**, no open task blocks it and no one else has it (one SQL
   statement: of N agents, one wins). A ready or to-do task that an open
   task blocks shows **Blocked**, and is ready again by itself when its
   blocker is done. **To do** means "not ready yet" (not claimable). An
   epic is never claimed; a task "in progress" needs someone on it (epics
   and stories may be in progress with no one).
   - Recommendation: **keep** (built).
3. **Roles under the level.** The level (D-28) decides whether an agent may
   act at all; under it, the pool's **lead** plans epics and stories, gives
   tasks to others and changes or cancels any task; a **member** makes
   tasks, works the ones it holds (in review, done, blocked, give it back),
   and edits the ones it made that no one holds. This is the research's
   permission table (`pools-messaging-tasks.md` §2.3).
   - Risk: a pool with no lead can't give tasks out through agents (the user
     still can, on the task). Alternative: everyone may assign (then every
     assignment asks the user anyway, by D-28).
   - Recommendation: **keep** (built).
   - *Changed by D-44 (2026-10-27):* the user no longer gives tasks out in
     the app; a pool with no lead gives work out through what the user
     tells its agents, and the user frees a task whose holder's session
     ended.
4. **A session for a task.** "Start a session for it…" asks for launch
   settings (and so the account), lists them in name order, and picks
   nothing for the user except the project's own default account when the
   user set one; nothing ranks by usage (D-31). The session starts in the
   pool's project with the task's brief as its first prompt: as the tool's
   own first prompt in a terminal (so its words are in that process's
   arguments, visible to `ps` on this Mac, like a pack's pointer prompt),
   or as the user's first message for a session run in AI Juggler. The
   task is then that session's, in progress, linked "started for".
   - Options: (a) as built; (b) a pointer only ("work on aj-7f3c; read it
     with task_get"), so no task words are in a process's arguments, at the
     cost of needing the agent tools on.
   - Recommendation: **(a)**, revisit after the tasks lab.
5. **Issue titles through the user's own `gh`.** A GitHub issue or pull
   request link's title and state are looked up only on the user's click
   ("Look up its title"), by running the user's installed `gh api` with
   the owner, repository and number; AI Juggler never reads `gh`'s sign-in
   and opens no connection of its own. Linear links show their id from the
   link. No sync.
   - Options: (a) `gh` on click (built); (b) AI Juggler calls
     api.github.com itself (a new network address on the Privacy page, and
     no private repositories without a token); (c) no titles, links only.
   - Recommendation: **(a)**.
6. **The Claude Code plugin outside AI Juggler's launches (AJ-E1905).**
   E1905 asks for the bridge "even outside AI Juggler's launches", but
   ADR-0020 gives a session its identity only with a launch (a key in its
   environment), and D-30 says sessions started elsewhere can't call the
   tools until continued in AI Juggler. Built: the plugin (a skill for
   working tasks, and the bridge as its server) which, in a session AI
   Juggler started, lists no tools (the launch already added them) and,
   anywhere else, answers each call with why it can't.
   - What the user sees today: installing it adds the skill; outside AI
     Juggler's sessions the tools say "This session wasn't started by AI
     Juggler with its agent tools…".
   - Options: (a) publish as built (the skill's value now; the tools'
     value only inside AI Juggler's sessions, where they're already there);
     (b) a new way in: a session started elsewhere asks AI Juggler to join
     its pool, the user approves it once in the app, and a key reaches that
     session without argv or a file (a new ADR: how the key gets there is
     the hard part); (c) skill only, no server.
   - Recommendation: **don't publish yet; decide (b) or (c) after the tasks
     lab.** The bundle and its install test are in the repo; publishing it
     with the app's releases is a release-workflow change for the release
     PR once ruled.
7. **Smaller points, built as stated:** the pool's list shows tasks (the
   work), and epics and stories are in **Plan**, behind its own fold; a
   card's task says **None** when its member holds none (known, not
   "Unavailable"); messages "about" a task are the threads whose words name
   its id (`aj-7f3c`), no extra link to keep; tasks never travel to other
   Macs until AJ-E1705; schema **v36**, with v35 kept for D-41's sightings
   (`e24-processes`): merge that branch first, or replace the empty `V35`
   with its migration when it lands (a database made on `e19-tasks` alone
   is a test's).

### D-44 Tasks are the agents' to write; the user sees, tracks and has four basic actions — SETTLED (2026-10-27)

**Ruling.** The maintainer, 2026-10-27: "these tasks are not really meant
for humans to write and control, no humans do it manually any more. They
are meant for sync and orchestration between AI Agents, and for humans to
see and track the tasks. Humans will manage tasks indirectly through
different communication and prompting with AI Agents. So yes we want to
show the tasks and display them, but our tool is not meant to write or
edit tasks. We might be able to do some basic management."

Asked which basic actions stay, the maintainer kept all four:

1. **Approve or send back** a task an agent marked **in review**:
   **Approve** makes it done; **Send back…** puts it in progress again
   with the same holder and a short note (at most 500 characters, needed),
   which the holder gets as the user's message (E18: at its next turn when
   it runs in AI Juggler, in its inbox when it runs in a terminal) and
   reads in the task's history (`task_get`). A task in review that no one
   holds goes back to ready.
2. **Free a stuck task**: a task whose holder's sessions have all ended or
   crashed (as the sessions' own records say: none starting, running,
   waiting on the user or stuck) loses its holder, so another agent can
   claim it; in progress goes back to ready, any other status stays. A
   holder that still runs isn't freed: the user asks it to give the task
   back.
3. **Cancel a task**: any open task, after the user confirms.
4. **Start a session for this task**: as built (D-31, D-43 point 4); the
   user picks the launch settings, and so the account.

**What changed (E19, AJ-E1903).**

- Removed from the app: the **New task** form and its entry points;
  changing any field of a task (title, what it asks, when it's done,
  priority, parent, issue link); free status changes (the **Status**
  menu); giving a task to someone (the **With** menu: a member, you or no
  one); adding or removing links. Links stay visible, read only, and the
  section shows only when a task has any.
- Removed from the backend: the window's `tasks_create`, `tasks_edit`,
  `tasks_assign` and `tasks_link` commands, `views::{create, edit, assign,
  link}`, and their wire types (`TaskInput`, `TaskEdit`). The agents' own
  tools are unchanged (`task_create`, `task_update`, `task_claim`,
  `task_assign` for leads, a held change answered in Needs you, D-43
  point 1).
- Added: one command per action (`tasks_approve`, `tasks_send_back`,
  `tasks_free`, `tasks_cancel`), each checked by the store against the task
  as it is when it's done (`TasksRepo::user_action`, the rules in
  `juggler_core::tasks::may_user`) and logged as one event in its history
  as the user's (`approved`, `sent_back` with the note, `freed` with the
  holder's name, `cancelled`). A task's page reads which ones it offers
  (`TaskDetail.actions`) from the same rules. An agent reads the user's
  actions as "the user", never "You".
- Kept: every read (the list, ready first; the counts; **Plan**; a
  session's **Tasks** tab; a task in full with its history), **Look up its
  title** for an issue link, and answering the agents' held changes and
  session requests in **Needs you** (oversight).

**Why.** Agents keep the tasks to stay in step with each other; the user's
part is to see the work and steer it by talking to the agents. Editing
from the app would make the window a second, competing author of the
agents' plan. The four actions are the decisions only the user makes (is
it done; is a stuck holder gone; is the work still wanted; which account
runs it), and each is narrow enough to be checked server-side, so the app
has no "edit anything" path.

- Risk: a user who wants a quick fix to a task's words now asks an agent
  to make it. A task held by **You** from before (the old **With** menu)
  can still be freed or cancelled.
- Evidence: `crates/juggler-app/tests/tasks_oversight.rs` (each action's
  rules, the note reaching a terminal and a driven holder),
  `tasks_guardrails.rs::the_app_writes_no_task`, the store's
  `the_users_actions_move_only_from_their_state_and_log_one_event`, vitest
  `ui/src/pages/tasks/tasks.test.tsx`; the gallery's `task-review-*`.

### D-45 Your own launch and switch sheets may suggest an account by its room — SETTLED (2026-10-27)

**Ruling.** The maintainer, 2026-10-27, asked whether their own launch and
switch sheets may rank accounts by remaining usage and mark one
"Suggested", given D-31 forbids quota-based picks: "Yes, keep suggestions."

**What it means.** D-31 governs what agents and tasks do: an agent's or a
task's assignment never reads quota, and nothing starts a session but the
user. It does not govern the user's own sheets: there, accounts may be
ranked by the room they have left and one marked "Suggested", because the
user always makes the choice. The spec gap audit of 2026-10-27 had raised
the overlap; this records the boundary.

### D-46 First run offers the optional Figurehead account again — SETTLED (2026-10-27)

**Ruling.** The maintainer, 2026-10-27, told that the optional "Connect a
Figurehead account" step left first run with the D-25 redesign (commit
91b640d6) while SPEC §12.1 and AJ-E1203 still describe it: "Bring the step
back."

**What it means.** First run ends with the optional, skippable step from
D-18 and AJ-E1203: **Connect account** and **Continue without an account**
as equal buttons, nothing pre-selected, and no reminder afterwards (D-18:
no pop-ups, banners or nagging). Everything works without an account; the
account's home stays Settings › Figurehead & Cloud.

### D-47 Open now stays put: Your order, pins, drag to reorder and an order menu — SETTLED (2026-10-27)

**Ruling.** The maintainer, testing 0.1.8: "the order the sessions and chat
windows are shown … seem to be bouncing just too much … by default users
want more stability … maybe we have PIN and re-order … some users might
want it sorted by some latest event, but I think those will be in
minority, and we can have a button / setting for them." Then, 2026-10-27:
"Approved: build the stable order, pin, drag-to-reorder and the order menu
into 0.2."

**Decision.**

1. **Your order is the default.** A row keeps its position. A new session
   joins the end of its group, in the order it was first seen. A state
   change (Working, Idle, Asks you, Done) changes only the dot and the word.
   An ended or closed session frees its slot; the rows under it close up,
   nothing else moves.
2. **A session that needs you stays in its place** in Open now, its state in
   amber; **Needs you lists it too**, as a shortcut at the top. This replaces
   the rule "Open now: not in Needs you" (AJ-E2103 Req.4). Each list and the
   badge count a session once; "Open now n" and Home's "n open" count every
   open session, the ones that need you among them.
3. **No automatic cut in Your order.** A place shows all its open rows; past
   8, the first 8 by the user's order then "+n more", never re-picked by
   activity. A place folds only when the user folds it (remembered).
   Inactive sessions still wait behind "Also: n inactive". AJ-E2104 Req.4's
   "last 3, then +n quiet" and the quiet fold stay in Latest activity only
   (a recorded deviation there).
4. **Pins** (AJ-E1502) lead their group in every order and work for every
   session: an AI Juggler session's pin is the session's own (as before); a
   session another app runs is pinned in AI Juggler's own record, never in
   the app's files. Pin is on the row's menu and on the row's hover and
   focus.
5. **Drag to reorder**, rows within their place and places among places,
   by a handle shown on hover and focus. ⌥⌘↑ / ⌥⌘↓ (Ctrl+Alt off a Mac, as
   ⌘ is Ctrl there) is the keyboard twin, said in a polite live region
   ("Moved to position 2 of 5"). ⌘1–9 follow the new place order.
6. **The order menu** on Open now's header ("Your order ▾") offers Your
   order, Latest activity (today's behaviour, held still under the pointer)
   and Name (A to Z), then Reset my order (with Undo). Settings › General ›
   Appearance offers the same choice.
7. **Kept** in AI Juggler's settings (`app_settings` key `rail_order`, one
   JSON value; no schema change), so restarts and backups keep the order,
   pins, place order, folds and the chosen order. Rows are kept by the
   rail's own keys (`s:<session>`, `c:<account>/<conversation>`); a key
   unseen for 30 days is a session gone for good and is pruned.

- Rejected: re-sorting by activity by default (the bounce); a new table
  (the settings store already backs up and restores); a drag library (a
  few pointer handlers do it, and the rail is first paint).
- Risk: a user who liked the activity order must pick Latest activity once.
  Ctrl+Alt+↑/↓ is a workspace switch on some Linux desktops; the platform
  key seam can rebind it when Linux ships.
- Evidence: vitest `ui/src/lib/rail-order.test.ts` (the order in each mode,
  moves, reset, pruning, the record round trip),
  `ui/src/components/rail-order.test.tsx` (the menu, the keyboard twin,
  pins, folds, Needs you mirrored), the gallery's `95-rail-order` captures.

### D-48 One Subagents view: the session's older list retired into the tab, and one source for the count — SETTLED (2026-10-27)

**Ruling.** The maintainer, 2026-10-27: "Yes, fold the Overview's older
Subagents list into the new Subagents tab and retire the old one."

**What changed (AJ-E1520, 0.2).**

- Removed: the "Subagents" disclosure in the session's notes
  (`SubagentsSection`), its `subtasks_list` command (and the dev bridge's
  route and the IPC audit's allow line), the app's `features::subtasks`
  (its tick and its wire types `SubtasksInfo`, `SubtaskInfo`,
  `SubtaskStateInfo`).
- In its place, one quiet line in the session's notes when it started
  any, "6 subagents · 2 running › Subagents", which opens the tab
  (wireframe screen 05, approved 2026-10-13, names the count in the
  session's inspector). The list itself is only the tab's.
- The row's count (I-08, `subtask_count`: the rail's "+N agents", the
  card's badge, the row's words) is now read by the same adapter readers
  as the tab (`HarnessAdapter::subagent_reader`), so every tool that lists
  subagents gets one: Claude Code, Codex, Kimi Code, Grok Build and
  OpenCode, not only the first two. It counts what the tab calls running:
  nested subagents and ones started in the background included, until
  their records say they ended. Before, a background Claude subagent and
  a subagent's own subagents weren't counted. Still `null` until read and
  once the session ends (D-06). The rail no longer falls back to the
  session's child processes when the count is `null`.
- Kept: `juggler_adapters::subtasks`, the start-and-end log the Claude
  Code and Codex readers are built on (one parser, not two).

**Why.** Two lists of the same subagents, from two parsers, could
disagree (a background subagent was "in the background" in one and
"Running" in the other), and the count only knew two tools. One reader
per session now answers the row and the tab.

- Evidence: `crates/juggler-app/src/features/subagents/count.rs` (unit),
  `crates/juggler-app/tests/subagents_count.rs`, vitest
  `ui/src/pages/session/subagents-tab.test.tsx` and
  `ui/src/lib/rail-model.test.ts`; the gallery's `88-subagents`.

### D-49 Your menu and snoozes: My Macs with an account; a snoozed need leaves the count — SETTLED (2026-10-27)

**Rulings.** The maintainer, 2026-10-27, on the notification system and
your menu (AJ-E2501..E2507):

1. "Yes: keep This Mac and Processes in the avatar menu, and show My Macs
   only when a Figurehead account is connected." Without an account your
   menu's second group is Settings, This Mac and Processes; with one, My
   Macs stays in the account group, next to Cloud backup.
2. "Yes: snoozed items leave the Needs you count until their snooze ends;
   muted ones stay counted." This changes the approved design's note
   ("snoozed and muted things are still counted in Needs you").

**What changed (E25).**

- Needs you is worked out in the backend (`features/notifications/needs.rs`)
  on the 5 s resources tick, window or not: one list and one count for the
  rail, Home, the bell, the app badge, the menu-bar item and the system
  notifications. The window works nothing out any more
  (`notifications_sync`, `NeedsSync` and the UI's computation are gone;
  `lib/needs-you.ts` reads the backend's list; the preview runs the same
  rules in a lazy stand-in, `preview/needs.tsx`).
- A snooze belongs to its subject (`need_snoozes`, schema v37:
  `s:<session>`, `c:<account>/<conversation>`, `signin:<account>`, …),
  until a time or until woken. The tick ends a snooze whose time has come
  and every view follows, so a snoozed need comes back into every count by
  itself. The rail's snoozes (Snooze for an hour, Mute this session) are
  these, no longer the window's own; the old ones move over once.
- The snooze toast says the need is out of Needs you until it is back
  ("Back at 9:00 tomorrow; out of Needs you until then."); a snoozed row
  says when it is back and offers **Back into Needs you now**.
- The rail's Needs you lists every need the bell counts (a sign-in, This
  Mac, a cap, an agent's message or task as compact rows under the
  sessions), so its number is the bell's.

**Why.** One count only holds if one place works it out, and only the
backend runs with no window (the badge and the menu-bar item); a snooze is
the user saying "not now", so counting it kept the badge lit for something
they had set aside, while a mute is about interruptions, not about whether
the thing still needs them.

- Risk: a snoozed item is out of sight in the count; the centre still
  lists it under **Snoozed: n**, and it comes back by itself.
- Evidence: `crates/juggler-app/tests/notification_model.rs`
  (`needs_you_is_worked_out_in_the_backend_with_no_window`: the rail's
  list, the bell, the badge and the menu-bar item from one tick; a snooze
  ending brings its need back), `centre/tests.rs`
  (`snoozed_needs_leave_the_count_until_they_wake_muted_ones_stay`),
  `juggler-core/tests/notices_storage.rs`, vitest
  `src/count-agreement.test.tsx` and `components/notifications.test.tsx`.

### D-50 ⌘N opens New session; New project is ⌘⇧N — SETTLED (2026-10-28)

**Ruling.** The maintainer, 2026-10-28, on the 0.2 walk's finding that ⌘N
opened New project while the screen spec (deep/11 §3, §4.8, §13) gives it
to New session: "⌘N opens New session (as the UX spec says), and New
project moves to ⌘⇧N."

**What changed.**

- ⌘N opens New session from anywhere, as the header's New session button
  and ⌘K's "New session…" do; ⌘⇧N opens New project (AJ-E305's flow (b),
  which had ⌘N until now). The shortcut registry (`ui/src/lib/shortcuts.ts`)
  holds both, so the ? sheet and ⌘K show the new keys.
- The notification centre (AJ-E2503) had ⌘⇧N. It moves to **⌥⌘N**, the
  nearest free key that still reads as "N for notifications" (matched by
  the key's place, since ⌥ changes the letter on a Mac); the bell's
  tooltip, the user guide and §4.14/§13 of the screen spec say so. *This
  consequence is the agent's, not the ruling's: the maintainer to
  confirm.*
- AJ-E305's text (E3) and AJ-E2503's (E25) point here.

**Why.** New session is the daily action and the spec's ⌘N; a new project
is rarer and is one Shift away. The ruling takes ⌘⇧N for New project, so
the centre needed another key; the bell stays one click away.

- Risk: muscle memory for ⌘N (New project) and ⌘⇧N (notifications) from
  0.1.x and the 0.2 nightlies; the ? sheet lists the keys.
- Evidence: vitest `src/app-keys.test.tsx` (the three keys open their own
  sheet and nothing else; the typing rule holds for each),
  `src/components/palette-commands.test.tsx`; Playwright
  `tests/visual/keyboard-flows.spec.ts` ("launch keys — ⌘N New session,
  ⌘⇧N New project, ⌥⌘N notifications (D-50)"), and flow (b) by ⌘⇧N in
  `new-project.spec.ts` and `a11y.spec.ts`.

### D-51 A switch offers this project's notes — SETTLED (2026-10-28)

**Ruling.** The maintainer, 2026-10-28, on the switch lab's finding that
Claude Code's auto memory doesn't travel (after A→B, account B no longer
had the codename memory A had saved): "Yes: the switch check offers to
copy this project's memory to the new account, the same way it offers
missing MCP servers and skills, and copies only with my approval."

**What changed.**

- A tool's per-project memory is an adapter capability
  (`HarnessAdapter::keeps_project_memory`, `project_memory`,
  `add_project_memory`): the adapter says whether its tool keeps notes
  for a project, and lists, reads and writes them through one shared
  module (`juggler_adapter::memory`). Claude Code implements it
  (`projects/<project>/memory/`: `MEMORY.md` and one file per note);
  no other tool does yet. Nothing outside the adapters names a tool.
- The switch's set-up check offers them as one more row of the same
  list, only when the source has notes for this project and the target
  lacks some: "Work Max's notes for this project (3) aren't on Personal:
  copy them?". It is never ticked for the user; nothing is copied
  unless they tick it.
- A copy adds only the missing notes, as new files only the user can
  read, and adds the index lines for what the target's index doesn't
  list yet, never one twice. A note the target has with other words is
  a conflict: kept as it is, and the row and the result say so. Only
  plain note files directly in the notes folder are read or written (no
  links, no `..`, no hidden files or subfolders).
- The copy is recorded like any set-up and undone from the same place:
  the undo takes out the index lines it added and the notes it made,
  only while each note is as it was made.
- A target whose tool keeps no notes (Codex) is offered them in the
  handover instead, the same row, unticked: "Work Max's notes for this
  project (2): include them in the handover?". *Built 2026-10-28 (it was
  left for later the same day, the agent's call):* the tick travels with
  the switch itself (`switch_execute`'s `approved`, the offer ids the
  switch carries out; the rest of the ticks are still set up beforehand),
  and the switch quotes each note's text in the handover file under its
  own heading, after the conversation and the rules files. Every note
  goes through the keys check the handover applies to quoted text; a note
  it finds a key, token or password in is left out whole, and the row and
  the handover say which. Nothing is written to the other tool's files,
  so there's nothing to undo; What's carried and the last step follow the
  tick (unticked, the notes stay with their own account).
- AJ-E1542's text (E15) points here; the user guide's switching page has
  **This project's notes**.

**Why.** The notes are what the tool learned about the project; a new
account that starts without them repeats questions the user already
answered. They are the user's own files on this Mac, like a skill, so
they are offered the same way, with the same undo.

- Risk: a copied note goes stale on one account while the other is
  edited; a later switch offers only the notes still missing, and says
  which differ.
- Evidence: `cargo test -p juggler-adapters --test claude_memory`
  (capability, merge, conflict, undo, private files, confinement),
  `cargo test -p juggler-app --test switch_memory` (offer, approval,
  apply, undo, a tool without notes), `--test switch_notes_handover`
  (the handover offer, the quote with the tick only, a note with a key
  left out), the `memory` unit tests in `juggler-adapter` and
  `juggler-app`; vitest `src/components/switch-notes.test.tsx`; live,
  the switching lab's notes suite (`just switch-lab-notes`, lab 21 in
  `docs/qa/evidence/AJ-E1539/lab-2026-10-28.md`): a planted note
  answered on B after the ticked copy, gone after its undo, and answered
  by Codex from the handover.

### D-52 Windows and Linux each get a dedicated first release, then ship in every release — SETTLED (2026-10-29)

**Ruling.** The maintainer, 2026-10-29: "Can you add in our global planner
of all the features and future versions, special releases for both Windows
and Linux separately." Earlier the same day, on why: "we have a lot of
specialized process monitoring, special desktop capabilities that work
separately for each OS … I am not saying we will not build them but I feel
they are massive and require their own projects and releases." And, on the
shape: "it is fine if its part of the same releases, but initial ones,
because we have to actually build and setup VM and test it and build
specialized capabilities for each, requires a dedicated release per OS."
and "Once we make the initial release than every feature is built all the
time for each OS."

**What changed.**

- **A dedicated first release per OS.** Windows and Linux each get one
  bring-up release: set up the VM and lab, build that OS's own
  capabilities (the OS seams), test it, and ship its first Beta. That is
  the per-OS project: [E26 — AI Juggler for Windows](../tasks/E26-windows-release.md)
  (AJ-E2601–E2616) and [E27 — AI Juggler for Linux](../tasks/E27-linux-release.md)
  (AJ-E2701–E2715), grounded in the readiness audit
  (`docs/research/cross-platform-readiness.md`). Its Beta gate is AJ-E2615
  / AJ-E2714.
- **Then every release.** After its first release an OS joins the same
  releases as macOS: one version stream (D-20 stands), the same channels
  (Nightly, Beta, Stable) and the same promotion rules (D-24). Every
  nightly builds, signs and publishes every shipped OS; one promote moves
  every OS's build of a version together; each channel keeps one feed with
  an entry per OS. There is no separate release line, version stream or
  channel position per OS, and no OS lags behind (AJ-E2616 / AJ-E2715).
- **Every feature on every shipped OS.** From an OS's first release on, a
  feature is built and tested for it all the time: the Definition of Done
  (`docs/plan/QUALITY.md` §2) says a task is done only when it works on
  every shipped OS, or is marked unsupported there with the reason, on
  screen and in the docs.
- Today macOS is the only OS shipped. Until an OS's first release, a macOS
  release never waits for its work and never claims it; CI's
  `build-windows` and `build-linux` jobs stay as compile guards for shared
  code (Linux also runs the workspace tests in `verify`).
- The plan's milestone table gains rows **W** and **L** beside M0–M8;
  M7-5 ("Windows/Linux bring-up begins") now keeps only the compile guards
  and points to E26 and E27 (`docs/plan/EPICS.md`). SPEC §1's platform
  row, §14's table and §15's decision 7, and `docs/RELEASING.md`, say the
  same.
- *The agents' reading, the maintainer to confirm:* once an OS ships, a
  `blocker` on any shipped OS stops the promotion of that version for
  every OS (D-24 applied across OSes), because every release ships every
  OS together.
- Left open for the maintainer in the first task of each epic (AJ-E2601,
  AJ-E2701): packaging (Windows: NSIS, MSI, MSIX or the Store, WinGet;
  Linux: AppImage, deb, rpm, Flatpak, Snap, and a package repository),
  signing, scope (architectures, Windows versions, WSL; Linux
  distributions, desktops and display servers), the Windows data folder,
  and which OS goes first and when. Windows' timing is settled by D-53
  (its first release comes before 1.0); Linux's stays open.
- The architecture rule is unchanged and now has a home per OS: OS code
  lives behind the OS seams (`juggler-platform`, the audited per-OS shims,
  the adapters), never as a per-OS branch in shared code; a seam shared by
  both systems is built once (E26's intro names each one's owner) and the
  platform guard lint's allowlist only shrinks.

**Why.** What makes AI Juggler useful (watching and stopping process
trees, finding and focusing desktop apps, keeping the computer awake, the
system's credential store, notifications, packaging and updates) is
different work on each system, and its first version needs a VM, a lab
and its own tests: too much to fold into a macOS release. Once that
foundation exists, one release flow keeps every OS current; separate
lines would let an OS fall behind.

- Risk: shared code drifts back to macOS-only assumptions before the
  bring-ups. The platform guard lint and the two compile guards stay
  required on every change, and the seams already built (the agent bridge,
  driven sessions and hand-over on `juggler-platform`) keep their honest
  "not on this system yet" answers.
- Risk: after an OS ships, its failures hold every release and every task
  costs more. That is the price of no OS lagging behind; the per-OS CI
  jobs must stay fast, and "unsupported on this OS, because …" is allowed
  only with its reason shown.
- Evidence: the two epic files; `python3 scripts/check_task_counts.py`
  (the catalog, the file map and the plan's count agree with the epic
  files and this log).

### D-53 1.0 comes only after a successful Windows release — SETTLED (2026-10-29)

**Ruling.** The maintainer, 2026-10-29, on the roadmap: "I think we should
only call version 1.0 after succesful Windows release".

**What changed.**

- The order is: macOS 0.x releases → Windows' dedicated first release
  (E26, D-52) passes its Beta gate (AJ-E2615) → only then 1.0. Windows'
  bring-up is no longer "after macOS 1.0" (SPEC §15 decision 7, as
  refined by D-52): it is a condition of 1.0.
- 1.0 (M8, Gate V8) therefore carries Windows as a shipped OS: its
  regression matrix and Stable channel cover macOS and Windows, built,
  signed and published by the same flow (D-52).
- Linux wasn't named: its first release (E27) stays planned with its
  timing open, and is not a condition of 1.0 unless the maintainer says
  so.
- Updated: SPEC §1 (platform row), §14 (the 1.0 row), §15 decision 7's
  note; `docs/plan/EPICS.md` rows M8 and W and M8-4's acceptance; E26 and
  E27's order-and-timing rows; the README's Windows and Linux lines.

**Why.** "1.0" is the promise that AI Juggler is complete and stable for
the people it's for; a version that can't run on Windows isn't that yet.

- Risk: 1.0 moves later by the length of the Windows bring-up. The macOS
  0.x line keeps shipping meanwhile (D-52: a macOS release never waits
  for Windows work), so users don't wait; only the number does.
- Evidence: this entry; `python3 scripts/check_task_counts.py`.

### D-54 Finding your way: Projects, Pools and Tasks in the header, a visible grouping switch, a Projects screen and a Tasks screen — SETTLED (2026-10-29)

**Ruling.** The maintainer, 2026-10-29, on the 0.2.0 walk: "Also where is
the projects ??? … We should have some way to group sessions also by
projects. maybe that is what we are missing a top filter or a tab or flag,
that switching grouping between : Account, App (current) and project …
Also Where is tasks/ epics ???" On the approved design
([`docs/design/finding-your-way-2026-10-29.html`](../design/finding-your-way-2026-10-29.html),
§1, §3 and §4): "Put the whole redesign in 0.3. Header: OK as proposed."

**What changed (E28, 0.3).**

- **The header** is **Home · Projects · Pools · Tasks · Accounts · Usage ·
  History** (was Home · Accounts · Usage · History: D-25 as amended by
  AJ-E1021 and AJ-E2507). The separate page title goes: the highlighted
  tab is the title (`aria-current="page"`, and the window's title says
  "AI Juggler · Projects"). A page below a screen (a project, a pool, a
  session, a subagent) names where it is in its own breadcrumb
  ("Projects › storefront"). Processes, This Mac and Settings stay in your
  menu (AJ-E2507). Keys: ⌘⇧P Projects, ⌘⇧O Pools, ⌘⇧T Tasks and ⌘⇧Y
  History, beside the existing ⌘0 Home, ⌘⇧A Accounts and ⌘⇧U Usage; each
  tab's tooltip names its key. *Corrected 2026-10-30 (the 0.3 polish):
  this line listed ⌘⇧H for History, but ⌘⇧H is Continue on another
  account; History takes ⌘⇧Y, a letter like the other tabs (numbered tabs
  would take ⌘1–9 from the list's places, AJ-E2104).* *The keys are the
  agents' choice (free in `ui/src/lib/shortcuts.ts`), the maintainer to
  confirm.* (AJ-E2803)
- **The rail's grouping is a visible switch**, App · Project · Account ·
  Pool, above Needs you, in place of the unlabelled icon and the View
  menu's Group by. App (the place: an app and its profile) stays the
  default (D-36). The switch remembers the choice per Mac. Pool shows
  once any pool has members besides a project's own (before, Group by
  Pool was greyed out "once you make a pool", though every project
  already has one, D-33). (AJ-E2802)
- **A Projects screen**: each project on one line, the same way: name,
  folder, what needs you, sessions working, open tasks, this week's
  tokens. Sorted by recent activity, searchable, paged. (AJ-E2804)
- **A project page with everything about it**: "Projects › storefront",
  its folder and branch, what asks you and what works, and the tabs
  **Overview · Sessions · Tasks · Pool · Subagents · Usage · History**
  (each with its count where it has one), Overview first with its four
  figures (open sessions and apps, tasks by state, this week's tokens,
  accounts used). Before: Sessions · Overview · Usage. (AJ-E2805)
- **A Tasks screen across every project**: by state (Ready, In progress,
  In review, Blocked; To do, Done and Cancelled one click away) as a
  Board, and the plan (epics → stories → tasks) as a second view,
  filtered by project, pool or holder, searchable. D-44 stands: agents
  write tasks; the screen shows and tracks them and offers only the four
  basic actions. Review items stay in Needs you too. (AJ-E2806)

**Why.** Things existed but had no door (the design's §0): grouping by
project, pool or account hid behind an unlabelled icon; projects had no
header entry (only that grouping or ⌘K); a pool's page opened from
Settings › Agent access or a Needs you card; tasks and epics lived only on
a pool's page. Folding the title into the tab makes room for three more
doors in a header that already wrapped at 1280 px.

- Risk: seven tabs, New session, the bell and the avatar at 1024 px. The
  header must not wrap at 1024×700 (the Tauri minimum is 1024×680); the
  nav's labels never truncate, and the gallery captures it at 1024 and
  1280 in both themes.
- Risk: a screen reader no longer hears a separate heading for the
  screen. Each screen keeps a visually hidden `<h1>` with its name, so
  the heading outline is unchanged.
- Risk: a list across every pool's tasks grows without end. It is paged
  by keyset cursor like the pool's own list (`tasks_list`), never
  loaded whole.
- Evidence: this entry and the approval record; E28's AJ-E2802–E2806,
  each with its evidence path under `docs/qa/evidence/<id>/`;
  `python3 scripts/check_task_counts.py`.

### D-55 A pool is one live canvas you watch and edit in place; a Pools screen; your layout is remembered — SETTLED (2026-10-29)

**Ruling.** The maintainer, 2026-10-29: "we need to have dedicated section
for pools. BEcause pool page should have dual purpose, a view of all of
the apps in pools, showing them, their statuses, and configuration of
pools. Pools requires a really very interactive view. I am not saying you
shouldn't have group by pools, keep that … if someone creates a pool, they
want to see interactions, how they configured in the pool, alerts within
the pool" and "In same place where you view you should create, edit
manage pool items and same view, there is no reason to have different
screens. Think how diagram dashboards are." On the approved design (§5):
"Pool canvas: I arrange nodes and it remembers, with an automatic first
layout."

**What changed (E28, 0.3).**

- **A Pools screen** (the header's Pools): a card per pool with a small
  map of its members, its name, how many members, what asks you and how
  many tasks move; a project's own pool says "(project pool)". New pool
  is on the screen. (AJ-E2807)
- **A pool's page is its canvas.** No separate view and setup screens:
  members are nodes (the session's name, Lead when it leads, account ·
  app, its state and what it's on); interactions are edges (a task given,
  messages, an ask), each drawn its own way and named for screen readers;
  alerts sit on the node or edge they concern, with their answers right
  there (an ask: Allow once · Always · Deny, the same answer as in Needs
  you, D-28; a held message: Deliver). The page's tabs are Canvas · Tasks ·
  Messages; the members and access tabs fold into the canvas and its
  inspector. (AJ-E2808)
- **Edit in place.** Drag a session from the palette ("Add a session")
  onto the canvas to add it, or onto empty space to start a new pool.
  Select the pool, a member or an edge and the inspector shows it and
  changes it: the pool's members and lead, what agents may do (D-28's
  level), delivery (D-29), the rules you granted (each one can be taken
  back), what notifies you, and a live feed; a member's tasks, messages,
  usage and rules; an edge's messages and tasks. Settings › Agent access
  stays the one page of every pool's levels and rules (D-28).
- **Your layout is remembered.** The first layout is automatic (the lead
  on top, members below, the same input always giving the same picture);
  nodes you drag stay where you put them, across restarts; a member who
  joins later is placed automatically without moving the ones you placed;
  "Arrange automatically" starts over, with Undo. *The agents' reading,
  the maintainer to confirm:* positions are kept per Mac, like Your order
  (D-47), since a pool that spans Macs (D-32) is drawn on screens of
  different sizes.
- **Group by Pool stays** in the rail (D-54's switch).
- The hard lines stand: only the user changes pools, levels and rules
  (D-28); adding a session to a pool never starts one, and agents never
  start sessions (D-31).

**Why.** Someone who makes a pool wants to see who is in it, what they do
to each other, how it is set up and what waits on them, and change it
where they see it, as a diagram dashboard does; two screens for the same
pool split that.

- Risk: the canvas is the largest piece of the redesign. It is built on
  the existing pool, message and task commands; what it adds is the
  layout store and the edges' read.
- Risk: dragging is not accessible to everyone (WCAG 2.2, 2.5.7 Dragging
  Movements). Every drag has a keyboard and menu equivalent (move a
  node with ⌥ and the arrows; "Add to this pool…" and "Move to pool…" in
  menus), and the canvas has a list view of the same members and edges.
- Risk: a busy pool draws many edges. Edges between the same two members
  merge into one with a count; the canvas holds 50 members within its
  frame budget.
- Evidence: the approval record; E28's AJ-E2807 and AJ-E2808, with their
  evidence under `docs/qa/evidence/<id>/`.

### D-56 Subagents, around what users want from them: what runs, what failed, what it cost — SETTLED (2026-10-29)

**Ruling.** The maintainer, 2026-10-29: "Subagents are really poorly
implemented … it is listing 437 subagents … it makes no sense to show all
of them as one. We need to have filters for currently active, history of,
search them … some of the subagents are indented ??? what does that mean
??? … if so we should have tree like lines. Also we should have better
breadcrumbs for subagent, also we should have section for subagent usage
and processes too … Remember these subagents need to be useful, for users,
think why would users want to view subagents in our app ?? Possibly to see
token usage, resource usage, what processes each subagent started. Also we
should have a summary of subagent is doing. maybe original prompt given."
On the approved design (§6, §7): "Put the whole redesign in 0.3."

**What changed (E28, 0.3).**

- **Why someone opens it** sets the order: something still running or
  stuck after the session looks idle; why the session is expensive; what
  got delegated and what failed; what a background subagent produced;
  which one started a process. Claude Code forgets finished subagents
  after 30 seconds and shows no cost per subagent, so the lasting record
  is AI Juggler's reason to show them.
- **The Subagents tab** opens with one line ("437 subagents · 3 running ·
  2 failed · 41% of this session's tokens") and the filters **Running ·
  Needs attention · All**, a search (task, prompt, result, file), Type and
  Sort; it opens on Running when anything runs. Subagents are **grouped
  under the message that started them** ("You asked: "fix the totals…" ·
  14:02 · 6 subagents · 2 running · 1.2M"); older groups fold. **Tree
  lines** (├ └ │) show who started whom, in place of the unexplained
  indent; a parent's tokens include its children's, its own on hover.
  Windowed and paged: never all 437 at once. (AJ-E2810)
- **A subagent's detail**, with a breadcrumb back through its parents
  ("Fix the cart totals › Subagents › Audit the cart store › Read the tax
  tables", Esc goes back): the task it was given (its whole prompt one
  click away), its result (or "It's still running…", or its failure in one
  line), its work (tool uses by kind, lines added and removed, files, its
  conversation), its usage (tokens, an estimated cost, its share of the
  session) and the processes it started, marked **probably** (matched from
  the commands it ran). Stop says honestly what AI Juggler can't do: it
  can't stop one subagent inside a Claude Code session it doesn't drive,
  and says how to in Claude Code. (AJ-E2811)
- **An incremental index.** The 437-subagent session has about 700 MB of
  subagent records, and today each page re-reads them (the readers are
  kept in memory for the last 8 sessions and lost on restart). A small
  saved index, updated as the files grow, makes the tab open at once and
  page. (AJ-E2809)
- **Honest data per tool**, said on screen where it differs (the design's
  §7): task, state, times and who started it are exact for every tool;
  prompt and result exact for Claude Code and Codex, where recorded for
  Kimi Code, Grok Build and OpenCode; tokens, model and cost exact for
  Claude Code, part of the session's total for Codex (shown as such),
  exact and separate where recorded for the others; tools, lines and files
  exact for Claude Code and derived for the others; processes "probably"
  everywhere; the CPU and memory of a subagent itself is not possible
  (every subagent runs inside its tool's one process), and the screen says
  so instead of showing a number.
- **No double counting**: a subagent's tokens are a split of its session's
  total and show as a share of it. **"Running" stays truthful**: a
  subagent with no end record after its session ends reads "Unknown",
  never a guess (D-06, as `SubagentStatus` already says); a quiet one
  shows its last activity.
- **Cross-links**: the session's Usage tab shows the subagents' share and
  their most expensive; its Processes tab names the subagent that probably
  started a process; the project page has a Subagents tab across its
  sessions. (AJ-E2812)
- The adapter contract grows (`juggler_adapter::subagents`): each tool's
  details, in its adapter only (ADR-0002), with the conformance kit
  checking that every adapter answers each new capability. D-48 stands:
  one Subagents view, one reader for the row's count and the tab.

**Why.** A flat list of 437 rows answers none of the questions people
open it with. What they want is what is still running, what failed, what
it cost, what it was asked and what it left behind, and the record
outlives the tool's own memory of it.

- Risk: a prompt or a result can hold a key or a password. Both are
  redacted (D-14) before they are indexed or shown, and the index keeps
  only what the screen shows.
- Risk: "probably" is wrong sometimes (two subagents ran the same
  command). It is a label, never a basis for an action: nothing is stopped
  because of the match, and the Processes tab stops processes as it does
  today.
- Risk: the index grows with every session. It is bounded (one row per
  subagent, prompt and result clipped), pruned with the session's own
  records, and rebuilt from the tools' files if it is lost.
- Evidence: the approval record; E28's AJ-E2809–E2812, with their
  evidence under `docs/qa/evidence/<id>/`.

### D-57 Every new or changed screen is critiqued for design quality, with shared controls and every popover captured — SETTLED (2026-10-29)

**Ruling.** The maintainer, 2026-10-29, on the rail's View menu: "this
screen is to bland, I mean look at it, ugly what happened with UI/UX
quality reviews ??" On the approved design (§2, §8): "Put the whole
redesign in 0.3."

**What changed (E28, 0.3).**

- **A design critique is part of done.** Every new or changed screen gets
  a short written critique next to its screenshots on the gallery's review
  page, against the app's best screens (the row menus, the rail's order
  menu, Usage and Processes), before the maintainer sees it. Its
  checklist: one alignment grid; a clear hierarchy (section titles,
  options, hints); consistent spacing; icons on menu and action lines; the
  shared controls, no system select, radio or checkbox; disabled states
  that say why; plain words, no internal ids; body 14 px and nothing under
  12 px; both themes, at 1280×800 and 1024×700. What it finds is fixed
  before the screen is called done. `docs/plan/QUALITY.md` §2 has the
  line. (AJ-E2814)
- **Shared controls** (AJ-E2801): one **choice line** (icon, label, hint
  under it, ✓ on the right), promoted from the order menu, for the View
  menu, the stop sheet's "What to stop" and Settings' choices; one
  **checkbox** and one **radio** in the accent colour with a clear
  disabled state, replacing the mix of system blue and purple in about 38
  files; one **picker** (the value and a chevron, opening a menu with ✓)
  replacing about 21 system selects (How it runs, History's filters, Open
  projects in, Add account). *The agents' addition:* a lint keeps native
  ones from coming back, with an allowlist that only shrinks.
- **The View menu, rebuilt** with them (AJ-E2802): Show (Quiet ones,
  Ended today, Subagents, Other Macs) and Only (App, Account, Project,
  each a sub-menu, never a system select), every line icon · label · hint
  · ✓, and Reset only when something changed. Group by moved to the rail's
  switch (D-54).
- **Every popover is captured** in the gallery, so none escapes review
  again; one that can't be is listed in the gallery's not-captured list
  with the reason.
- **The sweep's smaller fixes** (AJ-E2813): the limit dialog's orphan line
  and missing ×; New session's wrapped "Suggested" and text triangles;
  the tool menu covering its own button; Settings › General's squeezed
  card; Home's misaligned app buttons.

**Why.** Our gates checked "unchanged" (the visual baselines) and
"accessible" (the a11y gate), never "good": the View menu passed both
with two hint layouts, 12 px everywhere, system radios and selects and a
disabled option that looked selected, and it wasn't even captured.

- Risk: a critique is a judgement. The checklist and the named reference
  screens anchor it; the maintainer's review of the gallery stays the
  last word.
- Risk: replacing the native checkboxes, radios and selects (about 38
  files and 21 selects) can break keyboard and screen-reader behaviour
  the browser gave for free. The
  shared controls carry the roles and keys (radio groups move with the
  arrows, a picker is a menu button), and the a11y gate runs over every
  touched screen.
- Evidence: `docs/plan/QUALITY.md` §2; E28's AJ-E2801, AJ-E2802, AJ-E2813
  and AJ-E2814, with their evidence under `docs/qa/evidence/<id>/`.

### D-58 One local cache, sized by the machine, invalidated by stamp, the user's to clear — SETTLED (2026-10-30)

**Ruling.** The maintainer, 2026-10-30: "remember our tool is installed
on users machines. We are not exporting that data. So if you need to
store certain data for local caching purposes so our app is faster or
more capable, so be it. We can always options in settings where users
can disable that if they choose or if they want clear cache. But we
should be caching as much as possible to provide smooth experience, but
at the same time we need to have solid invalidation system of updating
it and managing. Like if our cache files get too big, our system should
automatically monitor that. Our system is going to be used on multiple
types of machines some lots of resource some very little so our system
should be smart about the usage of resources."

**What changed (0.3, ADR-0023).**

- **One cache system** (`features/cache`): a registry of local caches
  behind one trait, each with its plain name, place, size, and whether it
  can be trimmed an entry at a time or only whole. Registered: subagent
  lists, app icons, the search index. Never sent, every entry rebuilt
  from its source, owner-only, redacted first (D-14).
- **Invalidation by stamp:** each entry names its cache, format, the
  settings that shaped it and its source; any difference or a corrupt
  file drops it. Turning Last messages off drops the cached subagent
  prompts and results at once.
- **Sized by the machine:** 2 % of free disk between 64 MB and 2 GB (or a
  fixed 250 MB, 1 GB, 4 GB); under 5 GB free (or 5 % of a small disk) it
  trims to 64 MB and writes nothing new; memory caches scale with the
  Mac's memory. A cheap background pass a minute, least recently used
  first across caches; the whole-only search index only on low disk.
- **Settings › Advanced › Local cache:** the switch (off removes the
  files), the limit, Clear cache… (it says what gets slower) and each
  cache's size. Privacy's wording and claims say exactly what is kept.

**Why.** Three features kept files their own way, none measured
together, none aware of the machine, and Settings said "kept nowhere"
about text the subagent index kept.

- Risk: a cache that outlives a format or a setting shows stale words.
  Every read checks the stamp and drops a mismatch; a setting change
  purges what it shaped at once.
- Risk: monitoring costs more than it saves on a small Mac. One disk
  reading a minute; folder walks every 30 minutes or soon after writes,
  bounded per pass.
- Evidence: `crates/juggler-app/src/features/cache/tests.rs`,
  `crates/juggler-app/tests/local_cache.rs`; the gallery's
  `98-local-cache`.

### D-60 Deleting launch settings keeps their sessions — SETTLED (2026-10-30, 0.3.1)

**Ruling.** A data-loss bug found while writing E29 (AJ-E2907's check of
`ProfileRow.tsx`): one click on the bin deleted launch settings, and the
sessions table's `ON DELETE CASCADE` took every session made with them
out of History. The maintainer, 2026-10-30: "any bugs I find in 0.3
should got to 0.3.*" (D-59), so the fix ships in 0.3.1. Removing an
account already kept its history (AJ-E1501, v30); launch settings now
work the same way.

**What changed (0.3.1).** v39 adds `launch_recipes.removed_at`. Deleting
marks it: the settings leave every list and start nothing, History still
names them, and their usage footprints go. While a session made with
them is open the delete is refused, naming it. The bin asks first and
says the sessions stay.

**Why.** History is the user's record of their work; nothing a user
does to tidy their setup should erase it.

- Risk: a removed row lingers forever. It is a few hundred bytes per
  launch setting, the same trade AJ-E1501 made for accounts.
- Evidence: `crates/juggler-core/tests/storage_roundtrip.rs`
  ("profile_update_and_remove"), `crates/juggler-app/tests/manage_accounts.rs`
  ("deleting_launch_settings_keeps_their_sessions_and_waits_for_open_ones"),
  `ui/src/screens/accounts/delete-launch-settings.test.tsx`, the
  gallery's `41-policy`.

### D-63 Open starts an app that isn't running; Keep alive per app and profile — SETTLED (2026-10-31, 0.3.3)

*D-61 and D-62 (0.4's pools and designs) are recorded on the 0.4 line; this one ships first, in 0.3.3.*

**Ruling.** The maintainer, 2026-10-31: "Clicking on the open in the app
should start the app if its not running", and each app gets a setting
"like a keep alive" that restarts it when it isn't running, crashed or
was shut down. Investigated first: it was never built. `desktop_launch`
(AJ-E413) had no caller, Home's Open and the rail's only brought a running
app forward, and crash restarts covered only AI Juggler's own sessions
(AJ-E1509). "Both for 0.3.3."

**What changed (0.3.3).**
- AI Juggler remembers the desktop apps and profiles it sees running
  (`features::places`; the live view's full scan, at most once a minute):
  the bundle, the instance folder it was given, the sessions open in it.
  At most 32; forgotten after 90 days unseen unless kept alive. The
  window's setting writer can't change the record (it names a program).
- **Open** starts one that isn't running, on the same profile, in front:
  Home (a tile for each kept-alive app that's down), Settings › Desktop
  apps, an ended session's page ("Open in {app}"; Codex opens the thread
  by its link).
- **Keep alive** (Settings › Desktop apps, the rail's place menu) starts
  it again when it stops, behind the app in front. SPEC §1's third
  recorded amendment says when it never does.

**Why.** The desktop is where the work runs; an app that quit or crashed
unnoticed is work stopped. A relaunch changes no account and no content:
it is the app the user had open, the way they had it.

- Risk: an app whose executable was swapped is started by AI Juggler.
  Every start checks the bundle is still that app's and its executable
  is one no other user can change (`juggler-platform::file_trust`).
- Risk: a relaunch fights the user (they quit it on purpose). It is off
  until turned on per app; every start is a notice with the way to turn
  it off; three in 10 minutes pause it; a stop in AI Juggler pauses it.
- Not done: a desktop app's own sign-in isn't read (its data is the
  app's); a signed-out profile starts and shows its own sign-in.
- Evidence: `crates/juggler-app/src/features/places/` (tests),
  `crates/juggler-platform/src/apps.rs`,
  `ui/src/screens/settings/keep-alive.test.tsx`, the hand walk in
  `docs/qa/0.3.3-walk.md`.
