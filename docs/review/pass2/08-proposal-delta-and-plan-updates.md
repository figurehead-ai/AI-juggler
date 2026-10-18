# Pass 2 — Proposal Delta and Plan Updates (Synthesis)

> Second pass, synthesis-ready. Canonical input:
> `docs/expected-worfklow.md:1` (21 lines, maintainer example).
> Prior research cited, not restated: Maximum-Depth Report
> `docs/review/AI-Juggler-Maximum-Depth-Report-and-Proposal.md`
> (6,739 lines; cited as MD §/Part), `docs/review/deep/`
> 10-product-proposal (cited as D10 §), and
> `docs/review/full-competitive-ux-review.md` (cited as FCUR §).
> Baseline: `docs/PROPOSAL.md:1`, `docs/SPEC.md:1`,
> `docs/plan/EPICS.md:1`. Pass-2 siblings 01–07 in this folder
> are the evidence base; this file only synthesises their deltas.
> Terminology: **Provider / Account / Instance** per
> `docs/expected-worfklow.md:11`; legacy terms appear only in
> the migration map (§3.4) or in quoted prior text.

## 0. How to use this file in the maintainer meeting

Read §1 for positioning (5 min), §2 for the feature delta
(20 min), §5 for decisions that block Phase 0 (15 min).
§§3–4 are the exact edits to make once §5 is decided.
Every PRD in §2 is implementable without re-reading 01–07;
line citations let any reviewer falsify any claim in one hop.
RICE reuses the D10 §25 formula verbatim — Reach (0–10
dogfood personas) × Impact (0.25–3) × Confidence % ÷ Effort
(person-weeks) — so pass-1 and pass-2 scores are comparable.
Effort is T-shirt converted: S≈1 wk, M≈2 wk, L≈4 wk.

What this pass changed in one sentence: prior research made
quota the product (MD Executive decisions 1–6; D10 §25 ranks
Quota+forecast RICE 600 as Must); the maintainer workflow at
`docs/expected-worfklow.md:5`–`:7`, `:14`, `:17` makes
*switching, seeing, and cleaning up* the product, with quota
as one input to all three. §§1–2 enact that re-centring.

## 1. Revised product thesis and positioning

### 1.1 Thesis (replaces PROPOSAL §1 framing)

Prior thesis (`docs/PROPOSAL.md:10`–`:16`): a supervisor that
does profiles/parallelism, quota dashboards, and handover.
That ordering is the error pass 2 corrects: it lists quota
second and handover third as if they were peers, when the
maintainer workflow makes handover the *daily motion*
(`docs/expected-worfklow.md:7`, "as many times I want …
back and forth") and quota merely its trigger
(`docs/expected-worfklow.md:5`, "if I ran out of credits").

**Revised thesis.** AI Juggler is the switching layer for
AI coding work: it lets a developer start on any Provider
Account, exhaust it, and continue the *same work* — chat,
rules, project state — on any other Account or Provider,
unlimited times in both directions, without manual
configuration (`docs/expected-worfklow.md:6`,`:9`), while
always showing which Provider/Account/model is running
where and owning every process an Instance spawns until
it is verifiably gone (`docs/expected-worfklow.md:13`–`:18`).

Three load-bearing differences from the prior thesis:

1. **Switching is a loop, not an event.** Prior handover is
   A→B→A (one return; `docs/SPEC.md:288`; D10 §9 reset queue).
   The maintainer requires an unbounded chain (§2, F-N1/F-U1;
   evidence: pass2/02 §§2–6, pass2/01 R5 = Missing).
2. **Visibility is a surface, not a dashboard page.** The
   right inspector on selection (`docs/expected-worfklow.md:14`)
   — not the Overview page (`docs/SPEC.md:345`) — is where
   quota, resources, children, and history compose (§2, F-N3).
3. **Ownership outlives the session.** Processes, ports, and
   dev services an Instance spawned are Juggler's problem
   after close (`docs/expected-worfklow.md:17`–`:18`); prior
   SPEC §5 covers only live trees (`docs/SPEC.md:234`) (§2, F-N2).

### 1.2 Positioning sentence options (pick one; §5 D1)

| # | Sentence | Optimises for | Risk |
|---|---|---|---|
| A (recommended) | "Start anywhere. Switch when you hit the wall. Never lose the thread — or leave a zombie behind." | The maintainer loop (`:5`,`:7`,`:17`) | "Zombie" needs the §2 F-N2 glossary on first use |
| B | "One console for every Provider, Account, and Instance you juggle." | Naming mandate (`:11`); category creation | Says nothing about switching, the differentiator |
| C | "The quota-aware switchboard for multi-subscription AI coding." | D10 §1 quota wedge; searchability | Re-centres quota — the pass-1 error this file corrects |
| D | "Keep coding through every limit: your work follows you across tools and accounts." | Outcome language for non-experts | Longer; weak on visibility/cleanup pillars |

Recommendation: **A as tagline, B as the descriptive subtitle**
("AI Juggler — one console for every Provider, Account, and
Instance…"), C retired to SEO/docs metadata only. Rationale:
FCUR F7.1 already establishes "nobody owns juggling"; A claims
the *verb* (switching) rather than the noun (console), which
cc-switch-class tools (`docs/PROPOSAL.md:120`) can also claim.
Prior naming work (D10 §23) proposed positioning around the
quota/inbox wedge; pass 2 explicitly supersedes that choice.

### 1.3 What does not change

Compliance guardrails stand verbatim (`docs/PROPOSAL.md:61`–
`:68`; `docs/SPEC.md:26`): user-initiated execution only, no
proxying, no login automation, no autonomous rotation. Pass 2
adds no exception; §2's "zero-touch carry" means *pre-staged
and one-click*, never auto-executed (see §2 preamble). Fleet
(`docs/SPEC.md:362`), MIT licence (`docs/SPEC.md:466`), and
macOS-first sequencing are likewise untouched by this delta.

## 2. Feature delta vs prior Part 10 (D10)

Reading key: **NEW** = absent from D10 §§1–20; **UPGRADED** =
in D10 but scope materially increased by pass-2 evidence;
**DEMOTED** = in D10, now deferred or shrunk. Each entry is a
mini-PRD: Persona / Flow / Controls / Backend / Acceptance /
Effort / Priority / RICE. Personas reuse D10 §0 (P1 Multi-Maxer
≈ `docs/PROPOSAL.md:51` Persona A; P2 Work/Life Splitter `:52`;
P3 Multi-Toolist `:53`; P4 Operator `:54`) plus pass2/07 W1–W11
actors where noted. "Provider" below = harness vendor/tool
(Claude Code, Codex, ZCode); see §3.4 for the rename contract.

### 2.0 NEW features (5)

#### F-N1. Portable Context Pack as the canonical switch artifact — P0

- **Persona:** P1/P3 mid-task at a quota wall
  (`docs/expected-worfklow.md:5`–`:6`); pass2/07 W5, W6, W9.
- **Flow:** On "Switch…" the app assembles the pack in the
  background (pre-staged on quota-warning, not on click);
  user picks target Account/Provider, sees a one-line fidelity
  badge (T1–T4, `docs/SPEC.md:269`), clicks Switch. No file
  handling, no path entry, no manual rule copying.
- **Controls:** Switch button (session + inspector), target
  list with quota headroom, "What travels" expandable audit
  view (optional, never a gate), lineage breadcrumb A→B→C.
- **Backend:** Pack schema v2 (pass2/02 §4): conversation,
  rules/guidelines, project state (branch, diff, todos),
  MCP/settings references, lineage chain, redaction manifest.
  Extends `docs/SPEC.md:292` pack format; T4 pack becomes the
  *staging/preview artifact for every tier*, not only fallback.
- **Acceptance:** Pack assembles in <5 s for a 500-turn session;
  switch completes ≤2 clicks; fixture test proves rules +
  history + todos present on target for all 3 Providers;
  redaction manifest lists every withheld item (closes FCUR
  F1.6, which D10 left as a P1 note without an artifact).
- **Effort / Priority / RICE:** L (4 wk) · **P0** · R10×I3×C70÷4
  = **525**. Lower confidence than D10 §1 (80%) because T2/T3
  fidelity is unproven per FCUR F3.1–F3.3; that is priced in.

#### F-N2. Process Cleanup Centre (zombie & orphan ownership) — P0

- **Persona:** P4 Operator; any user whose closed Instance left
  a dev server holding a port (`docs/expected-worfklow.md:17`).
- **Flow:** Every spawn is ledgered at launch; on Instance
  close, crash, or app restart, survivors surface in Cleanup
  Centre grouped by owning Instance with age, CPU/RAM, ports.
  User verifies (process tree + listening ports + cwd) then
  cleans one, a group, or all; opt-in auto-cleanup tiers
  (pass2/05 §10) apply only to ledger-owned, verified-dead-owner
  processes.
- **Controls:** Cleanup Centre view (badge = orphan count),
  per-group Verify / Clean / Keep, "Clean all safe" (ledger-owned
  only), auto-cleanup tier selector (Off / Notify / Clean-safe).
- **Backend:** Spawn ledger (new persistent table; pass2/05 §5),
  ppid-map + port-occupancy map (§7), re-adoption on restart
  extending `docs/SPEC.md:224`,`:251`; SIGTERM→SIGKILL ladder
  reused from `docs/SPEC.md:225` with per-group confirmation.
- **Acceptance:** Fixture harness spawning child + grandchild +
  port-listener: after parent kill, all three detected ≤10 s,
  attributed to correct Instance, cleaned with zero collateral
  (unrelated process with same binary name untouched); audit
  log entry per action (satisfies D10 §17 for this surface).
- **Effort / Priority / RICE:** L (4 wk) · **P0** · R9×I3×C80÷4
  = **540**. Prior coverage was one bullet ("zombie/orphan
  detection", `docs/SPEC.md:224`) — pass2/05 §§2–3 shows pass 1
  conflated zombies, orphans, and leaked dev services; this
  PRD is the correction, not a restatement.

#### F-N3. Right Inspector (selection snapshot surface) — P0

- **Persona:** All; the daily driver at
  `docs/expected-worfklow.md:14`–`:15`.
- **Flow:** Selecting any session, Instance, Provider, or
  Account opens the right inspector: identity (Provider /
  Account / model), quota headroom + reset, live resources
  (CPU/RAM/tree), children/subprocesses, session history,
  quality/performance signals, and contextual actions
  (Switch, Stop, Clean children, Resume lineage).
- **Controls:** Inspector sections (collapsible): Identity,
  Quota, Resources, Processes, History, Actions. Pin/compare
  two selections; keyboard: `]` cycles sections.
- **Backend:** Aggregation query over existing stores
  (`docs/SPEC.md:256` usage engine, §5.2 footprints) + spawn
  ledger (F-N2) + lineage store (F-U1). No new collection;
  this is a read-model — which is why it ships early (§4).
- **Acceptance:** Inspector renders <200 ms for any selection;
  every field labelled Measured/Computed/Estimated per
  `docs/SPEC.md:259`; empty states honest (no fabricated data,
  closing FCUR F2.4 on this surface); Playwright flow:
  select→switch→clean without leaving the inspector.
- **Effort / Priority / RICE:** M (2 wk) · **P0** · R10×I2×C90÷2
  = **900** — highest NEW score; prior SPEC instance panel
  (`docs/SPEC.md:347`) is a static subset with no quota,
  process, or history composition (pass2/04 §2).

#### F-N4. Indicator system (ambient Provider/Account/model/quota) — P0

- **Persona:** P1 glancing across 10+ Instances
  (`docs/expected-worfklow.md:13`); pass2/07 W3 morning triage.
- **Flow:** Every row/card/chip carries the same indicator
  grammar: Provider icon, Account label, model, status dot,
  quota micro-bar with reset time. Legend is one hover away;
  colour is never the only channel (icon + text redundancy).
- **Controls:** Density toggle (Comfortable/Compact), sort by
  quota-remaining / status / recency, filter by Provider /
  Account / needs-attention; tray aggregate mirrors the grammar
  (extends D10 §19 rather than inventing a second language).
- **Backend:** View-model only; consumes `docs/SPEC.md:257`
  outputs. Requires the *subscription-expiry clock* split from
  pass2/01 R2: window-reset vs plan-renewal are two indicators,
  never one conflated "quota" glyph.
- **Acceptance:** A/B screenshot test: user identifies
  Provider+Account+model+quota-state for 10 rows in <30 s;
  colourblind simulation passes; every indicator has a text
  equivalent (a11y gate from MD 01 §8 debt list).
- **Effort / Priority / RICE:** M (2 wk) · **P0** · R10×I2×C90÷2
  = **900** (tied F-N3; they ship as one surface, §4 Phase 1).

#### F-N5. Switch-back with divergence handling (Return+) — P0/P1

- **Persona:** P1 whose original Account window reset mid-detour
  (`docs/expected-worfklow.md:7`; `docs/SPEC.md:288`).
- **Flow:** On reset, "Switch back" is suggested (never executed).
  If the detour Instance diverged (new turns, file edits), the
  user sees a divergence summary — turns added, files touched,
  todos changed — and chooses: resume original primed with the
  delta, or stay. N-hop chains behave identically at every hop.
- **Controls:** Switch-back banner + inspector action, divergence
  diff view, "while you were away" delta preview (editable).
- **Backend:** Lineage chain entity (F-U1) + delta renderer over
  Transcript IR (`docs/SPEC.md:185`); generalises the single-hop
  delta at `docs/SPEC.md:288` to arbitrary chain depth.
- **Acceptance:** 5-switch same-day fixture (pass2/01 R5 delta):
  context-integrity assertions at every hop; switch-back after
  divergent edits presents the diff and never silently merges;
  source Instance remains resumable throughout (T2 copies,
  `docs/SPEC.md:287` invariant preserved).
- **Effort / Priority / RICE:** M (2 wk) · **P0** for 1-hop,
  **P1** for N-hop divergence UI · R8×I3×C70÷2 = **840**.
  Prior art covers only the 1-hop happy path; the divergence
  case — the one that loses work — was unmodelled (pass2/02 §6).

### 2.1 UPGRADED features (scope increased; 5)

#### F-U1. Handover → Unlimited Switching Engine (was D10-adjacent, SPEC §7) — P0

Upgrade: one-way takeover (D10 §9 reset queue; `docs/SPEC.md:287`)
becomes a bidirectional Switch Graph (pass2/02 §§2–3): three
named switch kinds — Switch-Account (same Provider), Switch-
Provider (cross-tool), Switch-Instance (resume elsewhere) —
each with its own fidelity tier, budget (≤2 clicks, pre-staged),
and metric. Lineage promotes from a `HandedOver` status flag to
a chain entity supporting N hops with per-hop integrity checks.
Backend: switch executor (freeze→stage→verify→launch, pass2/02
§5) + lineage store + pack v2 (F-N1). Acceptance: the 5-switch
fixture (F-N5) plus per-kind latency budgets. Effort L (shared
with F-N1; incremental 2 wk) · **P0** · R10×I3×C70÷2 = **1050**
(counting only incremental effort; do not double-count F-N1).

#### F-U2. Smart Onboarding → Provider/Account auto-discovery (was SPEC §12.1) — P0

Upgrade: the Claude-only scan (`docs/SPEC.md:414`–`:420`,
ADR-0006) becomes a cross-Provider recommender (pass2/03 §§3–5):
scan all Providers, infer plan/Account identity with confidence
labels, propose a ready-to-launch *juggling set*, zero user-named
settings (anti-configuration budget, pass2/01 R6). New-Account,
new-Project, and add-Provider flows specified button-by-button
(pass2/03 §6). Acceptance: fresh-machine → 2 Accounts parallel
<5 min (`docs/PROPOSAL.md:142`) measured per Provider, not only
Claude; isolation verification checklist (pass2/03 §8) green.
Effort L (4 wk) · **P0** · R10×I2×C80÷4 = **400** — low RICE,
still P0: it is the top-of-funnel gate for every other score.

#### F-U3. Usage engine → Quota + subscription-expiry + forecast (was D10 §1) — P0, rescoped

Upgrade in kind, not just size: split the two clocks (window
reset vs plan renewal/expiry, pass2/01 R2), add per-model burn
attribution (maintainer: "models currently running",
`docs/expected-worfklow.md:13`), and feed *switch pre-staging*
(pre-assemble F-N1 pack at warning threshold) — quota stops
being a dashboard and becomes switching's trigger. Forecast
honesty rules unchanged (`docs/SPEC.md:259`). Acceptance adds:
expiry indicator fires ≥7 days before renewal; forecast MAPE
reported per window type (D10 §26 kill-test retained).
Effort L (4 wk, as D10) · **P0** · RICE **600** (D10 score kept;
pass 2 changes its *role*, not its rank).

#### F-U4. Resource manager → per-Instance process ownership (was SPEC §5.2) — P0

Upgrade: live-tree monitoring (`docs/SPEC.md:234`) gains spawn-
time ownership (ledger, F-N2), port occupancy, post-close
survival detection, and hog attribution ("which Instance's
child is eating 3 GB") before any alarm (pass2/05 §§6–8).
Admission advisor (`docs/SPEC.md:237`) now reads ledger history
including leaked-process cost. Acceptance: F-N2 fixture plus
advisor verdict within ±20% *including* leaked children.
Effort M incremental (2 wk beyond SPEC §5.2) · **P0** ·
R8×I2×C80÷2 = **640**.

#### F-U5. Project context sync → zero-touch carry (was SPEC §7.2 flow 4) — P1

Upgrade: diff-preview-as-gate (`docs/SPEC.md:289`) becomes
background sync with audit view: rules, memory, todos, and
MCP/settings references carry automatically on switch (F-N1
manifest records what moved); preview remains as an optional
inspect/undo surface. This directly reverses the prior
inspectable-over-invisible emphasis flagged at pass2/01 R4.
Guardrail-safe: sync is part of the user-clicked switch, never
a background write to harness state (violates `docs/SPEC.md:68`
otherwise). Acceptance: switch with zero manual file steps;
undo restores prior rules byte-identically. Effort M (2 wk) ·
**P1** · R8×I2×C70÷2 = **560**. (Absorbs D10 §14 MCP/settings
sync, promoted from P2/Could — see §2.2.)

### 2.2 DEMOTED features (4, with reason and revisit trigger)

| ID | Was (D10) | Now | Reason (evidence) | Revisit when |
|---|---|---|---|---|
| D-1 Review/Diff Queue | §11, Could, RICE 120 | **Deferred post-1.0** (read-only diff in inspector only) | Demand was single-persona `[inferred]` (D10 §26); inspector History (F-N3) covers the daily need | 5 dogfood diaries request review routing (D10's own gate) |
| D-2 Fleet teaser/control | §20 Could; SPEC M6–M7 | **Schema-only in Phase 0; control post-1.0** | Switching/inspector/cleanup are unproven locally; fleet multiplies every failure (FCUR F6.4) | Local switch loop passes Gate V4-equivalent (§4) |
| D-3 Templates/Prompts Library | §16, Could, RICE 280 | **Cut from roadmap; community/plugin surface** | No maintainer-workflow sentence supports it (`docs/expected-worfklow.md:1`–`:21` silent); pack (F-N1) already carries prompts in context | Pack users export/share packs repeatedly (W9 signal) |
| D-4 Multi-repo grouping / Present mode | §15, Could, RICE 300 | **Deferred; grouping = filter only** | Agency Present-mode demand unverified (D10 §26 "invented" risk); W4 needs grouping-as-filter, not a mode | 3 agency interviews (D10 §30) confirm invoice/present value |

Explicitly *not* demoted despite low RICE: F-U2 onboarding
(gate for all reach), D10 §8 account health, §12 backup/restore,
§17 audit log — all retain D10's † blocker status and Phase-0/1
placement. Needs-Attention Inbox (D10 §2, RICE 1215) is not
demoted either: it is *absorbed* as the inspector + indicator
attention states (F-N3/F-N4); building it as a separate inbox
surface is what is cut. Command Palette (D10 §10) stays P1 but
moves after F-N3, since the inspector becomes its main target.

## 3. Exact document updates recommended

### 3.1 SPEC.md — sections to rewrite (in edit order)

1. **§3 Domain model** (`docs/SPEC.md:86`): add `Provider`
   (rename of `HarnessId`, `:91`), `Instance` as a first-class
   entity (today only `InstanceKind`, ADR-0007 decision 1),
   lineage-chain entity, spawn-ledger entity; demote `Profile`
   to an Account-scoped settings overlay (keep the type, remove
   it from user-facing language). Migration: additive schema,
   `kind` column already exists per ADR-0007 consequences.
2. **§5 Supervisor** (`docs/SPEC.md:214`): add spawn-ledger,
   post-close orphan detection, port map, Cleanup Centre
   contract (F-N2/F-U4); keep the no-SIGSTOP rule (`:225`).
3. **§6 Usage** (`docs/SPEC.md:254`): two-clock model (F-U3),
   per-model attribution, pre-staging trigger for F-N1.
4. **§7 Handover** (`docs/SPEC.md:261`): retitle "Switching";
   pack v2 as canonical artifact for all tiers; N-hop lineage;
   switch-back divergence (F-N5); zero-touch carry (F-U5).
   Preserve the click-to-execute invariant (`:263`) verbatim.
5. **§9 UX** (`docs/SPEC.md:316`): right inspector as the
   primary detail surface; indicator grammar; Cleanup Centre;
   reverse the "merge Instances away" direction (MD 01 §B-08 /
   FCUR F2.2) — Instances stay, per ADR-0007 and
   `docs/expected-worfklow.md:11`. Add subscription-expiry to
   Overview health strip (`:345`).
6. **§12.1 Onboarding** (`docs/SPEC.md:409`): generalise the
   Claude-only scan to the cross-Provider recommender (F-U2).

### 3.2 PROPOSAL.md — sections to rewrite

- §1 Summary (`docs/PROPOSAL.md:8`): adopt §1.1 thesis order
  (switch → see → clean; quota as trigger).
- §6.1–6.4: rename to Provider/Account/Instance language;
  §6.4 handover becomes "Switching (unlimited, bidirectional)".
- §6.5: add post-close ownership (Cleanup Centre) as a named
  capability; §9 metrics: add switch-budget (<2 clicks, <5 s
  pack), 5-switch continuity, orphan-detection ≤10 s targets
  alongside the existing A→B→A ≥80% criterion (`:144`).

### 3.3 EPICS.md — resequencing (see §4 for the full roadmap)

- Insert **Phase 0 (truth + blockers)** before M1 expansion:
  F4.1 credential-copy decision (FCUR §Q4), `guardrails.rs`,
  honest empty states, lineage + ledger schema migrations.
- Pull **F-N3+F-N4 (inspector/indicators)** into the first
  shippable slice — they are read-models over data M1–M3
  already collect; D10/FCUR ordering buried them behind quota.
- Move **handover honesty (T2 fixture-proof)** ahead of any
  T3/T4 claim (FCUR F3.1–F3.5); F-N1 pack ships as the honest
  T4-first path, T2/T3 graduate per-Provider behind fixtures.
- Push **fleet control** (M7 scope, `docs/plan/EPICS.md:136`)
  post-1.0 per D-2; keep M6 read-only telemetry schema early.
- Every gate (`docs/plan/EPICS.md:158`) gains two checks:
  switch-budget evidence and cleanup-collateral evidence.

### 3.4 ADR list and UI naming migration (Providers/Accounts/Instances)

New ADRs to write (numbering continues from ADR-0007):

| ADR | Title | Decides |
|---|---|---|
| 0008 | Provider/Account/Instance terminology | Canonical nouns; `HarnessId`→Provider (code rename or alias), Profile demoted to internal overlay, Session retained only as the conversation inside an Instance (ADR-0007 sense) |
| 0009 | Portable Context Pack v2 | Pack as canonical artifact for all tiers; schema, redaction manifest, lineage embedding |
| 0010 | Spawn ledger and process ownership | Ledger schema, ownership at spawn, cleanup safety tiers, never-touch-unowned rule |
| 0011 | Switch Graph and lineage chain | Three switch kinds, N-hop lineage, divergence/switch-back semantics |
| 0012 | Two-clock quota model | Window-reset vs subscription-expiry as separate domain concepts and indicators |

**Naming migration (UI first, code second, never big-bang):**
Phase A (one release): UI strings switch to Provider/Account/
Instance; routes and component names aliased. Phase B: domain
types renamed with serialisation-compatible aliases
(`HarnessId` kept as deprecated alias for one minor version).
Phase C: docs/SPEC/EPICS swept; `Profile` survives only in
settings-overlay contexts. Conflicts to flag in ADR-0008:
(i) code `Profile` vs maintainer's "profiles are complicated"
(`docs/expected-worfklow.md:9`) — the word itself is the burden;
(ii) prior "delete Instances" (MD 01 §B-08) vs ADR-0007 +
maintainer mandate — maintainer wins; (iii) D10 §23 naming
proposal (quota-wedge language) superseded by §1.2 above.

## 4. Revised phased roadmap and 90-day plan

Phases replace D10 §25 sequencing and resequence
`docs/plan/EPICS.md:22` milestones; milestone IDs are kept for
tracking continuity, scopes change as noted. Durations are
targets for one small team; parallelise only where §2 RICE
dependencies allow (quota → pre-staging; ledger → cleanup).

| Phase | Window | Ships | Gate (evidence required) |
|---|---|---|---|
| 0 Truth | Wk 1–2 | ADR-0008–0012 drafted; lineage + ledger schema; F4.1 decision; honest empty states; guardrail tests | ADRs accepted; migrations round-trip (`docs/plan/EPICS.md:44` discipline) |
| 1 See | Wk 3–6 | F-N4 indicators + F-N3 inspector (read-model); two-clock quota display (F-U3 display half); naming Phase A | Inspector <200 ms; indicator A/B test (F-N4 acceptance); zero fabricated fields |
| 2 Switch | Wk 5–10 (overlap) | F-N1 pack v2 (T4-first), F-U1 Switch-Account, F-N5 1-hop switch-back, F-U5 zero-touch carry; F-U2 onboarding generalisation begins | 5-switch fixture green for Claude; ≤2-click budget timed; source-resumable invariant proven |
| 3 Clean | Wk 8–12 | F-N2 Cleanup Centre + F-U4 ownership; auto-cleanup tiers; audit log | Collateral-damage fixture (F-N2) green; orphan detection ≤10 s |
| 4 Widen | Post-90d | Switch-Provider (T3) per-Provider graduation, N-hop divergence UI, fleet read-only telemetry | Per-Provider fidelity fixtures; T3 labelled honestly until then |

**90-day plan (weeks, owner = role not person).** Wk 1: §5
decisions D1–D4 taken; ADR-0008/0010 drafted (Architect).
Wk 2: schema migrations + guardrails (Core). Wk 3–4: indicator
grammar + inspector skeleton on live data (UI+Core). Wk 5:
quota two-clock display + pre-staging trigger (Core). Wk 6:
**Gate: See** — dogfood week starts (all). Wk 7–8: pack v2 +
Switch-Account end-to-end (Core+Adapters). Wk 9: switch-back
1-hop + divergence summary (Core). Wk 10: **Gate: Switch** —
5-switch dogfood script, continuity judged ≥80% (target from
`docs/PROPOSAL.md:144`, now applied per-hop). Wk 11: Cleanup
Centre + ledger backfill (Supervisor). Wk 12: **Gate: Clean** +
v0.1-beta scope freeze; fleet-control and D-1/D-4 stay out.
Slip rule (from D10 §27): if the spine slips, cut Phase-4
widening and D-1 read-only diff first — never the fixtures.

What moves vs EPICS today: M3 quota (`docs/plan/EPICS.md:86`)
splits — display half joins Phase 1, forecast half stays;
M4 handover (`:99`) becomes Phase 2 with T4-first honesty;
fleet M6/M7 (`:126`,`:136`) become schema-early/control-late;
M5 release engineering (`:113`) gates at Phase-3 exit.

## 5. Open decisions for the maintainer

| # | Question | Options | Recommended default | Cost of delay |
|---|---|---|---|---|
| D1 | Positioning sentence (§1.2)? | A / B / C / D | **A** (tagline) + B (subtitle) | Low — blocks only marketing copy; decide by Wk 2 |
| D2 | Does "Profile" survive in UI? | Keep / demote to Settings / remove | **Demote** to settings-overlay wording (ADR-0008) | Medium — every week adds UI strings to migrate |
| D3 | Cleanup auto-tier default? | Off / Notify-only / Clean-safe | **Notify-only**; Clean-safe opt-in per Account | High if wrong either way: Off hides F-N2's value, Clean-safe risks collateral trust |
| D4 | T3 cross-Provider launch claim? | Ship labelled-beta / hold until fixtures | **Labelled beta**, fidelity badge mandatory (F-N1) | Medium — holding delays P3 wedge; shipping unlabelled repeats FCUR F3.1 |
| D5 | Subscription-expiry data source? | User-entered renewal date / inferred from plan / hidden until known | **User-entered, optional**, inferred only as "unknown" (never guessed) | Low — indicator degrades honestly; guessing would break `docs/SPEC.md:259` |
| D6 | Fleet control timing? | Keep M7 pre-1.0 / schema-only till post-1.0 | **Schema-only** (D-2) | High — pre-1.0 control starves Phases 1–3 of the same engineers |
| D7 | Pack redaction default? | Withhold secrets always / user-toggle | **Always withhold**, manifest shows what (F-N1) | Low to decide, high to reverse after a leak |

## 6. Discussion agenda — maintainer meeting (60 min)

1. **(5 min) Thesis check.** Does §1.1's re-centring (switch →
   see → clean; quota as trigger) match intent at
   `docs/expected-worfklow.md:5`–`:18`? If not, which pillar
   is overweighted? *Outcome: thesis adopted or amended.*
2. **(10 min) Decisions D2, D3, D4** (§5) — the three that
   change code this month. D1/D5–D7 can be async. *Outcome:
   three defaults locked; ADR-0008/0010 owners named.*
3. **(15 min) Switch-budget walkthrough.** Live-talk the
   F-N1/F-N5 flow against a current build: count clicks,
   find where manual steps hide (pass2/01 R4). *Outcome:
   budget (≤2 clicks, <5 s pack) confirmed or revised.*
4. **(10 min) Cleanup safety demo.** F-N2 fixture story:
   what "verified owned" means, what is never touched.
   *Outcome: auto-tier default (D3) set with eyes open.*
5. **(10 min) Roadmap cut line.** Walk §4 slip rule: confirm
   Phase-4/fleet-control are the cut candidates, not
   fixtures or gates. *Outcome: 90-day scope frozen.*
6. **(10 min) Naming sweep timing.** Phase A UI rename —
   next release or the one after? (ADR-0008.) *Outcome:
   release assigned; Instances-tab reversal confirmed
   against MD 01 §B-08.*

Parking lot (do not open unless time remains): monetisation
(D10 §24 — unchanged by this pass), desktop-Provider adapters
(ADR-0007 decision 5), stable-channel timing (`docs/SPEC.md:476`
open item 2). Next artefact after this meeting: ADR-0008 and
ADR-0010 drafts within one week, then SPEC §3/§7 rewrites (§3.1).

---
*Evidence note.* Pass-2 siblings 01–07 carry the primary
research (external URLs, forum/GitHub evidence, competitor
teardowns); this synthesis cites them by section and adds no
new external claims, so no new URLs are introduced here.
Any claim above without a `file:line`, prior-section, or
sibling-section citation should be treated as opinion and
challenged in the meeting (§6, item 1).



