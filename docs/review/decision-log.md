# AI Juggler — Decision Log

> Status: **Final** — marked by the maintainer's instruction on
> 2026-10-13 ("Implement all as spec-ed out, I already approved those
> decisions with the other agent"), which also authorises Stage 1
> (documents) and the staged implementation that follows per D-15.
> Source evidence: `docs/review/AI-Juggler-Maximum-Depth-Report-and-Proposal.md`,
> `docs/review/AI-Juggler-Expected-Workflow-Second-Pass.md`, and
> `docs/expected-worfklow.md`.

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

## Unresolved (in discussion order)

- None. D-01..D-15 were marked Final on maintainer instruction
  (2026-10-13); D-16 and D-17 were settled by maintainer instruction
  on 2026-10-13 and follow the same law.
