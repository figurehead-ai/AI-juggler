# 12 — Roadmap, Metrics, Risks & GTM: A Sequencing Plan That Sells the Juggle

> **Scope:** Revised epics, quality gates, beta programme, measurement,
> pricing/sustainability, go-to-market, risk register, decision log,
> and a 90-day week-by-week plan.
> **Companions:** [full-competitive-ux-review.md](../full-competitive-ux-review.md) ·
> [EPICS.md](../../plan/EPICS.md) · [QUALITY.md](../../plan/QUALITY.md) ·
> [RUNBOOK.md](../../plan/RUNBOOK.md) · [gate-M1.md](../../qa/gate-M1.md)
> **Date:** 2026-10-13 · **Status:** Proposal — no scope is authorised
> until the maintainer closes the decision log in §10.
> **Citation rule:** Every factual claim about this repo cites
> `file:line`; every competitor claim cites a primary URL. Forward-
> looking targets are labelled **[target]** and are not evidence.

---

## TL;DR — the decision this document asks for

1. **Stop sequencing by harness count; sequence by loop closure.**
   The founding promise is G2/G3 — know before the quota wall and
   never lose momentum ([PROPOSAL.md:32-33](../../PROPOSAL.md#L32)) —
   yet quota sits at M3 after M2 ships two adapters
   ([EPICS.md:26-28](../../plan/EPICS.md#L26),
   [SPEC.md:452-454](../../SPEC.md#L452)). The full review calls this
   inverted ([full-competitive-ux-review.md:15-19](../full-competitive-ux-review.md#L15)).
   This plan pulls Claude-Measured quota, stuck/waiting emission,
   and handover-honesty ahead of adapter #2.
2. **v0.1-beta is a Claude-only juggling loop, not a three-harness
   board.** One harness with quota + honest handover + return-on-
   reset beats three harnesses that are quota-blind. ZCode — bundle-
   only and undocumented ([full-competitive-ux-review.md:416-418](../full-competitive-ux-review.md#L416))
   — is the last adapter, not the second.
3. **The sunset window is the GTM window.** Vibe Kanban is
   sunsetting after Bloop's 2026-04-10 shutdown and Crystal was
   deprecated Feb 2026 ([full-competitive-ux-review.md:50-52](../full-competitive-ux-review.md#L50)).
   A migration path for those users is cheaper than any paid channel
   and expires; it must ship with v0.1-beta, not after 1.0.
4. **No metrics without an event taxonomy the code can actually
   emit.** §7 defines events, properties, and the exact funnel from
   install → 2 accounts → quota wall → handover → return. If an
   event cannot be produced by a Tauri command that exists or is
   scoped in §3, it is marked **[not yet emittable]**.
5. **Stay MIT, stay free, monetise nothing until 1.0 proves
   retention.** The licence is MIT by ADR
   ([docs/adr/0005-mit-license.md](../../adr/0005-mit-license.md#L1));
   sustainability options in §8 are sequenced so none of them
   requires relicensing or a cloud account — both would break the
   local-first invariant ([PROPOSAL.md:67](../../PROPOSAL.md#L67)).

---

## 0. Baseline: what is actually true today

Do not plan from the milestone table; plan from the gate file and
the changelog. The two disagree, and that disagreement is itself
the first roadmap item.

| Claim | Evidence | Implication |
|---|---|---|
| README says "spec phase / no releases yet" | [README.md:7](../../README.md#L7) | Public face understates the product |
| CHANGELOG records shipped handover v0 + Claude adapter | [CHANGELOG.md:335-341](../../../CHANGELOG.md#L335) | Handover exists but is transcript-only |
| Gate-M1 is mechanics-verified, awaiting user confirmation, all boxes unchecked | [gate-M1.md:3-4](../../qa/gate-M1.md#L3), [gate-M1.md:54-67](../../qa/gate-M1.md#L54) | M2 work starting now violates EPICS process |
| EPICS bars the next milestone without gate sign-off | [EPICS.md:16](../../plan/EPICS.md#L16) | Phase 0 is a process fix, not code |
| QUALITY requires the repo to tell the truth | [QUALITY.md:9](../../plan/QUALITY.md#L9) | README/CHANGELOG drift is a quality defect |
| Only Claude is registered; Codex/ZCode have zero adapters | [full-competitive-ux-review.md:369-370](../full-competitive-ux-review.md#L369) | "Multi-harness" is a type-level claim today |
| `Stuck/WaitingUser/QuotaBlocked` modelled, never emitted | [full-competitive-ux-review.md:30-32](../full-competitive-ux-review.md#L30) | The #1 daily question is unanswerable |
| Quota is M3, after M2 adapters | [SPEC.md:452-454](../../SPEC.md#L452) | Roadmap inverted vs. G2/G3 |

**Consequence:** Any revised epic that starts with "build adapter
#2" inherits three lies — that M1 is closed, that quota is a
dashboard feature rather than the product, and that handover is
done. Phase 0 exists to retire those lies before new scope lands.

---

## 1. The central sequencing decision: quota vs. adapters

### 1.1 Why this is the decision, not a detail

PROPOSAL G2 is "know before you go" and G3 is "never lose
momentum" ([PROPOSAL.md:32-33](../../PROPOSAL.md#L32)). Both require
quota state: the return-on-reset flow needs a reset time
([SPEC.md:288](../../SPEC.md#L288) via full review), and the
handover wizard needs target headroom
([EPICS.md:109](../../plan/EPICS.md#L109)). Without quota, handover
is a copy operation with no destination advice, the tray has
nothing to show, and the admission advisor (SPEC §5.2) has no
quota input. Adapters, by contrast, multiply surface: each one
imports its vendor's format fragility, and Codex quota is
Estimated with no API while ZCode is bundle-only
([full-competitive-ux-review.md:416-418](../full-competitive-ux-review.md#L416)).

### 1.2 Options considered

| Option | What ships first | Loop closed? | Risk |
|---|---|---|---|
| A. Adapters first (current EPICS M2→M3) | Codex + ZCode, then quota | No — 3 blind harnesses | Highest: fragility ×3 before the differentiator exists |
| B. Quota first, Claude only (recommended) | M3-1/M3-2 + stuck + handover-honest, then Codex | Yes, on one harness | Lowest: proves G2/G3 where the data is Measured |
| C. Quota + Codex in parallel | Statusline feed + Codex adapter concurrently | Partially — Codex quota stays Estimated | Splits a tiny team; conformance kit not ready |
| D. All three quotas, no new adapter | Claude Measured + ZCode Computed + Codex Estimated on paper | No — ZCode/Codex have no adapter to feed it | Quota without sessions is a chart, not a juggle |

Option D fails on inspection: M3-3/M3-4 (ZCode credit computation,
Codex estimation, [EPICS.md:92-94](../../plan/EPICS.md#L92)) presuppose
adapters that do not exist. Option C fails on staffing (§4): the
conformance kit must land *with* adapter #2, not after #3
([full-competitive-ux-review.md:421-423](../full-competitive-ux-review.md#L421)),
and building it concurrently with the quota engine doubles the
review surface per PR against the ≤400-line guideline
([QUALITY.md:26](../../plan/QUALITY.md#L26)).

### 1.3 Decision: Option B, with two carve-outs

- **Pull forward:** M3-1 Claude statusline quota feed
  ([EPICS.md:90](../../plan/EPICS.md#L90); mechanism at
  [SPEC.md:192](../../SPEC.md#L192)), M3-2 usage engine
  ([EPICS.md:91](../../plan/EPICS.md#L91)), M2-5 stuck detection
  ([EPICS.md:79](../../plan/EPICS.md#L79)), M4-2 lineage
  ([EPICS.md:104](../../plan/EPICS.md#L104)).
- **Defer:** M3-3/M3-4 until their adapters exist; ZCode adapter
  to the last pre-1.0 phase (documented surface beats bundle
  archaeology, [full-competitive-ux-review.md:466-468](../full-competitive-ux-review.md#L466)).
- **Keep early regardless:** the adapter conformance kit ships
  *with* Codex (Phase 2), because every later adapter — including
  community ones — inherits its fixture corpus.

### 1.4 What "quota first" must not become

Quota-first is not dashboard-first. The Usage screen is a
`ComingSoon` placeholder today
([full-competitive-ux-review.md:193-194](../full-competitive-ux-review.md#L193)),
and charting is L3 detail in the redesigned IA. The Phase 1 quota
deliverable is three glanceable surfaces — ring + burn-down +
reset countdown in NewSession, the handover wizard, and the tray
— each carrying its Measured/Computed/Estimated honesty label
([SPEC.md:254-259](../../SPEC.md#L254)). A beautiful Usage page
with no launch-time forecast would repeat the original inversion
in miniature.

---

## 2. Revised epics: Phase 0 → 1.0

Each phase lists entry criteria (all must be true before work
starts), scope, exit criteria (all must be evidenced in the gate
report per [QUALITY.md:86-109](../../plan/QUALITY.md#L86)), and
explicit non-goals. Phases replace M2–M8 numbering for planning;
EPICS task IDs are retained for traceability.

### Phase 0 — Truth & gates (days, not weeks)

**Entry:** Maintainer available for two decisions (D1, D2 in §10).
**Scope:**
- Fix README status + milestone table ([README.md:7](../../README.md#L7),
  [README.md:61-71](../../README.md#L61)); add a docs-consistency
  check (README status vs. latest CHANGELOG entry) to the gate
  checklist.
- Label handover v0 "transcript-only (experimental)" in UI and
  CHANGELOG ([CHANGELOG.md:335-341](../../../CHANGELOG.md#L335));
  downgrade advertised capabilities to `RESUME` only — `TRANSPLANT|
  FORK` are advertised but unimplemented
  ([full-competitive-ux-review.md:243-249](../full-competitive-ux-review.md#L243)).
- Decide the credential byte-copy contradiction: import copies
  `.credentials.json` into the managed dir
  ([full-competitive-ux-review.md:320-326](../full-competitive-ux-review.md#L320))
  against SPEC S8's "never copied" wording. Recommended: amend
  S8 to the 0700-managed-dir carve-out + guardrail test (§10, D1).
- Close or explicitly waive gate-M1 in the gate file itself,
  naming waived criteria ([gate-M1.md:54-67](../../qa/gate-M1.md#L54)).
- Replace fabricated Overview numbers (`percent={0}` rings,
  hardcoded "Machine OK") with honest empty states
  ([full-competitive-ux-review.md:193-199](../full-competitive-ux-review.md#L193)).
- App-level `loading|ready|degraded|error` model; delete every
  `window.alert` path ([full-competitive-ux-review.md:160-169](../full-competitive-ux-review.md#L160)).

**Exit:** `just verify` green; gate-M1 signed or waived in writing;
no numeric UI element renders invented data; README, CHANGELOG,
and EPICS agree on current state. **Non-goal:** any new feature.

### Phase 1 — v0.1-beta: the Claude juggling loop

**Entry:** Phase 0 exit + gate-M1 disposition recorded.
**Scope (the loop, nothing else):**
- Quota: M3-1 statusline feed + M3-2 usage engine; rings,
  burn-down forecast, reset countdown, honesty labels.
- Needs-attention slice: emit `Stuck/WaitingUser/QuotaBlocked`
  end-to-end (model exists at
  [full-competitive-ux-review.md:201-207](../full-competitive-ux-review.md#L201);
  emission choke point is `apply_lifecycle_event`), rail chips,
  inbox v1 sorted by waiting time.
- Handover-honest: use `transcript_ref` not newest-by-mtime,
  freeze source before copy, copy the whole session file set
  (subagents, file-history), verify the copy parses, record
  lineage; move logic into `juggler-handover` (currently a
  doc-only stub, [full-competitive-ux-review.md:391-395](../full-competitive-ux-review.md#L391));
  PTY-independent golden-path test so CI exercises it.
- Safety: `guardrails.rs` suite (named in SPEC but missing,
  [full-competitive-ux-review.md:350-357](../full-competitive-ux-review.md#L350));
  worktree-on-launch option + same-branch warning; DB
  backup/restore (single `juggler.sqlite` holds lineage + usage).
- Daily UX: ⌘K palette + session search; Resume/Relaunch on
  ended sessions; session archive/search v1; accessibility P1s
  (dialog semantics, ARIA tabs, text status chips, contrast).

**Exit (the dogfood criterion):** On real Claude, with 2 accounts:
launch → quota wall visible *before* exhaustion → handover with
fidelity label → return on reset. Plus: 20-session fake-harness
load test in CI; coverage targets met
([QUALITY.md:63](../../plan/QUALITY.md#L63)); signed + notarized
build installs cleanly on a fresh VM (existing M5-2 scope,
[EPICS.md:118](../../plan/EPICS.md#L118)). **Non-goals:** Codex,
ZCode, Fleet, review queue, cost ledger UI beyond per-session
tokens.

### Phase 2 — v0.2: second harness + cost (breadth with proof)

**Entry:** v0.1-beta shipped; ≥10 external users completed the dogfood loop (§5); no open P0/P1.
**Scope:** Codex adapter (M2-1/M2-2, [EPICS.md:75-76](../../plan/EPICS.md#L75)) **with** the conformance kit (same fixture corpus per adapter — launch/resume/parse/quota-tier; Codex quota = Estimated); cost ledger (tokens→USD via versioned pricing table mirroring cc-switch's models.dev sync, rollups + CSV, every figure tier-labelled; no USD view exists today, [full-competitive-ux-review.md:76-80](../full-competitive-ux-review.md#L76)); account health (tier/expiry/affinity warning, extending metadata-only `login_state`); reset-queue (passive timer + notification, resume still one click per ADR-0003); pack redaction; tray quick-switch; scale fixes (byte-offset tailing, shared `sysinfo`, joined queries — bounds at [full-competitive-ux-review.md:368-380](../full-competitive-ux-review.md#L368)).
**Exit:** Claude+Codex side-by-side; honesty tiers on every number; 20-session load test on both adapters; a contributor can add a fake adapter from the kit docs alone. **Non-goals:** ZCode, Fleet, auto-rotation.

### Phase 3 — v0.5: third harness + the review surface

**Entry:** v0.2 stable ≥4 weeks; conformance kit proven by Codex.
**Scope:** ZCode adapter (M2-3) behind the kit, with ZCode Computed quota (M3-3) + Codex calibration (M3-4) landing where their adapters exist; profile clone + selective MCP/rules sync; review queue **only if validated** — demand is **[inferred]** ([full-competitive-ux-review.md:502-505](../full-competitive-ux-review.md#L502)), so read-only diff first, inline-to-agent after ≥5 unprompted requests; quiet notifications; session replay (cf. [opcode.sh](https://opcode.sh/)); Vibe/Squad config import (§8.3); self-update hardening (bad-signature rejected, [EPICS.md:117](../../plan/EPICS.md#L117)).
**Exit:** Three harnesses, one board/inbox; ≥3 real migrants import unaided; update channel proven. Labelled **v0.5** = complete single-machine product; 1.0 reserved for the stability promise. **Non-goals:** Fleet control, Win/Linux packaging.

### Phase 4 — v1.0: fleet-readiness + stability, then fleet

**Entry:** v0.5 retention met (§6.3); security review scheduled.
**Scope:** Fleet link (M6: identity/mDNS/pairing/telemetry/read-only board, [EPICS.md:130-134](../../plan/EPICS.md#L130)) on the Phase-1 fleet-ready inbox model; fleet control (M7, originally v0.3 at [EPICS.md:136-145](../../plan/EPICS.md#L136)) is **post-1.0** — premature for a tiny team (§3). Stabilisation, perf pass, security review, docs completeness; stable channel opens at 1.0 ([PROPOSAL.md:150](../../PROPOSAL.md#L150)).

**Exit:** Full regression matrix green; no open P0/P1; stable
channel live; gate report signed. **Non-goal:** team/org
features — explicitly out of scope by design
([PROPOSAL.md:45](../../PROPOSAL.md#L45)).

### 2.1 Dependency graph (what blocks what)

```
Phase 0 ──► gate-M1 disposition ──► Phase 1 (all of it)
Phase 1 quota ──► handover headroom check ──► return-on-reset
Phase 1 status emission ──► inbox ──► tray badge ──► (v1.0) fleet roll-up
Phase 1 handover crate ──► Phase 3 cross-harness replay
Conformance kit (Ph.2) ──► ZCode (Ph.3) ──► any community adapter
Cost ledger (Ph.2) ──► budget alerts ──► Persona-A retention (§7)
Fleet identity (Ph.4) ──► fleet control (post-1.0 only)
```

The critical path is **quota → handover-honesty → return-on-
reset**. Everything else either feeds it or waits for it. If a
week slips, cut from Phase 1's Daily-UX list first — never from
the critical path, and never by skipping the fidelity test.

---

## 3. Staffing & effort realism for a tiny team

**Assumption [target]:** 1 maintainer + occasional contributors +
agent runs under [RUNBOOK.md](../../plan/RUNBOOK.md#L1). The
runbook's own loop — lowest open task, ≤400-line PRs, gate stops
for human confirmation ([RUNBOOK.md:23-30](../../plan/RUNBOOK.md#L23)) —
is the capacity model; this section prices phases against it.

| Phase | Engineering weeks [target] | Bottleneck | De-scope lever |
|---|---|---|---|
| Phase 0 | 0.5–1 | Maintainer decisions D1/D2 | Waive gate-M1 explicitly instead of closing it |
| Phase 1 | 6–9 | Fidelity test + status emission are serial | Cut ⌘K to session-search only; defer archive UI |
| Phase 2 | 5–7 | Codex format drift; pricing-table upkeep | Ship cost ledger Claude-only, Codex tokens-only |
| Phase 3 | 5–8 | ZCode bundle archaeology | ZCode read-only (sessions+quota, no handover) |
| Phase 4 | 6–10 | Fleet transport + security review | Ship 1.0 single-machine; fleet link as v1.1 |

Rules: one critical-path item in flight (parallel quota+handover PRs collide at `apply_lifecycle_event`); each phase pays its own test debt (Phase 1: `guardrails.rs` + PTY-independent golden path); budget 10–15% **[target]** vendor-drift tax — Claude ships weekly, Codex broke profiles in 0.134.0 ([PROPOSAL.md:132](../../PROPOSAL.md#L132)); book UI design checkpoints ([EPICS.md:55](../../plan/EPICS.md#L55)) on the critical path or the gate stalls.

## 4. Quality gates per phase

All phases inherit the QUALITY Definition of Done
([QUALITY.md:13-26](../../plan/QUALITY.md#L13)) and the gate
definition ([EPICS.md:158-164](../../plan/EPICS.md#L158)). The table
adds only phase-specific, falsifiable checks.

| Gate | Automated | Functional (evidenced) | Kill criterion |
|---|---|---|---|
| P0 | `just verify`; docs-consistency check | Gate-M1 signed/waived; no fabricated UI numbers (screenshot audit) | Any UI number without a data source |
| P1 (v0.1-beta) | Guardrails suite; PTY-independent handover test; 20-session load test; coverage ≥85% core ([QUALITY.md:63](../../plan/QUALITY.md#L63)) | Real-Claude dogfood: wall→handover→return; quota within tolerance vs. `/usage` across a full window (M3-6 criterion, [EPICS.md:95](../../plan/EPICS.md#L95)) | Handover loses subagent/todo state silently |
| P2 (v0.2) | Conformance kit passes both adapters; cost-ledger property tests on window math | Claude+Codex parallel day; CSV totals match fixture sums exactly | Any quota/cost figure without an honesty tier |
| P3 (v0.5) | Bad-signature update rejected; import fixtures for Vibe/Squad configs | ≥3 external migrants complete import unaided | Update path installs an unsigned build |
| P4 (1.0) | Full 3-OS matrix; fuzz with no new crashes since last gate ([QUALITY.md:95](../../plan/QUALITY.md#L95)) | Fresh-machine install → 2 parallel accounts < 5 min ([PROPOSAL.md:142](../../PROPOSAL.md#L142)) | Any open P0/P1; any guardrail regression |

A gate that fails its kill criterion does not get waived — it
gets a smaller phase. That is the lesson of gate-M1 sitting
unsigned while M2 scope accumulated
([full-competitive-ux-review.md:452-460](../full-competitive-ux-review.md#L452)).

---

## 5. Beta & dogfood programme

### 5.1 Who to recruit, in order

1. **Vibe Kanban refugees (highest intent, expiring).** Bloop shut
   down 2026-04-10; Vibe is community-maintained, last push
   2026-04-24 ([full-competitive-ux-review.md:646-653](../full-competitive-ux-review.md#L646));
   upstream: [github.com/BloopAI/vibe-kanban](https://github.com/BloopAI/vibe-kanban).
   They already run parallel agents in worktrees; Juggler's pitch
   is supervision + quota, not another board.
2. **Crystal / Nimbalyst switchers.** Crystal deprecated Feb 2026
   for commercial Nimbalyst
   ([full-competitive-ux-review.md:655-660](../full-competitive-ux-review.md#L655));
   upstream: [github.com/stravu/crystal](https://github.com/stravu/crystal).
   Users who chose an MIT tool once will choose one again.
3. **Claude Squad power users** who hit the quota wall the TUI
   cannot see ([github.com/smtg-ai/claude-squad](https://github.com/smtg-ai/claude-squad));
   **cc-switch / claude-swap users** who have accounts but no
   session supervision ([github.com/farion1231/cc-switch](https://github.com/farion1231/cc-switch),
   [github.com/realiti4/claude-swap](https://github.com/realiti4/claude-swap)).
4. **The maintainer's own 5-account setup** (gate-M1 dogfood
   environment, [gate-M1.md:33-43](../../qa/gate-M1.md#L33)) — the
   permanent canary, not a substitute for external users.

### 5.2 Programme mechanics
Cohorts of 5 **[target]** (named build, 30-min pairing call, one task: the dogfood loop) — small cohorts keep the tiny team (§3) from drowning. Recruit in Vibe/Crystal issues/Discussions, r/ClaudeAI, Claude Discord; offer the migration guide (§8.3) first. **Dogfood script:** install → import → 2 accounts parallel → run one to its 5h wall → handover (labelled) → return on reset; any step needing a terminal outside the app is a P1. Label every beta issue with the funnel stage it blocked (§6.3).

### 5.3 What beta must prove before Phase 2 entry

≥10 external users complete the dogfood loop; ≥3 complete a
return-on-reset unprompted; zero reports of writes outside
managed dirs (the PROPOSAL success criterion,
[PROPOSAL.md:145](../../PROPOSAL.md#L145)); handover continuity
judged successful in ≥80% of tries
([PROPOSAL.md:144](../../PROPOSAL.md#L144)).

---

## 6. Metrics: north star, funnel, retention

### 6.1 North star

**Weekly Juggling Loops Completed (WJLC):** a loop = one user
moves a live session across accounts (handover executed) *and*
the destination session produces output afterwards. It is the
only metric that requires G1+G2+G3 to all work at once. Stars,
downloads, and sessions launched are vanity inputs to it.

- **[target] v0.1-beta exit:** 10 external users × ≥1 loop/week.
- **[target] v0.2:** 50 WJLC/week; **v0.5:** 200; **1.0:** 500.
- Guardrail metric paired with it: **Handover Fidelity Complaint
  Rate** (loops followed within 7d by a fidelity issue) must stay
  <5% **[target]** — WJLC growing while fidelity complaints grow
  faster means the label, not the feature, is working.

### 6.2 Exact event taxonomy

Local-first means telemetry is opt-in only
([PROPOSAL.md:67](../../PROPOSAL.md#L67)); every event below is
emitted to the local `usage_events`-style store first and leaves
the machine only if the user opts in. Names are `snake_case`,
past tense, `object_action`. **[not yet emittable]** marks events
whose emitting code is scoped but unbuilt.

| Event | Emitted by | Key properties |
|---|---|---|
| `app_first_launch` | Tauri startup | `os, arch, app_version` |
| `account_imported` | import command | `harness, source_kind (t3/vibe/squad/manual), size_mb_bucket` |
| `account_login_confirmed` | `login_state` probe | `harness, method (vendor_terminal)` — never token data |
| `session_launched` | launch command | `harness, account_id_hash, worktree_bool, admission_verdict` |
| `quota_snapshot` | statusline feed | `harness, tier (measured/computed/estimated), pct_5h, pct_7d, resets_at` |
| `quota_wall_hit` **[not yet emittable]** | `QuotaBlocked` emission (Ph.1) | `harness, pct_at_hit, session_age_min` |
| `attention_raised` **[not yet emittable]** | status emission (Ph.1) | `kind (stuck/waiting/quota/crash), session_id_hash` |
| `attention_cleared` **[not yet emittable]** | inbox action | `kind, wait_min, action (reply/handover/resume)` |
| `handover_prepared` | handover wizard open | `from_tier, to_tier, fidelity_tier, headroom_pct` |
| `handover_executed` | handover command | `fidelity_tier, files_copied, lineage_depth` |
| `handover_returned` | resume-on-reset click | `away_duration_min, delta_sessions` |
| `session_resumed` | resume command | `harness, age_hours, via (board/palette/tray)` |
| `cost_exported` (Ph.2) | CSV export | `rows, range_days` |

Identity rule: `account_id_hash` is a salted local hash; no
email, path, transcript text, or credential-adjacent field is a
property of any event. This follows the metadata-only precedent
of `login_state` ([full-competitive-ux-review.md:323-326](../full-competitive-ux-review.md#L323)).

### 6.3 Funnel & retention

**Activation funnel (per install cohort):**
`app_first_launch → account_imported (≥1) → account_login_confirmed
(≥2 accounts) → session_launched (2 parallel) → quota_wall_hit →
handover_executed → handover_returned`. The 2-account step is
the activation cliff: PROPOSAL's own success bar is install →
2 parallel accounts < 5 min ([PROPOSAL.md:142](../../PROPOSAL.md#L142)).
**[target]** ≥40% of installs reach 2 parallel accounts in the
first session; <20% means onboarding — currently the best-built
surface ([full-competitive-ux-review.md:53-55](../full-competitive-ux-review.md#L53)) —
has regressed or the import path is broken.

**Retention:** W1/W4 cohort retention of users with ≥1 WJLC.
**[target]** W4 ≥35% for the juggler cohort; single-account users
retained mainly by quota visibility are a separate cohort and
must not be averaged in — they validate G2 alone. Churn signal
to instrument from v0.2: accounts imported but never launched in
14d (import succeeded, value never demonstrated).

## 7. Pricing & OSS sustainability

The code is MIT ([docs/adr/0005-mit-license.md](../../adr/0005-mit-license.md#L1));
signing is via Vuxtra's Apple Developer account with notarisation
budgeted ([PROPOSAL.md:135](../../PROPOSAL.md#L135)). Options:

| Option | Revenue | Invariant risk | Verdict |
|---|---|---|---|
| Pure OSS + GitHub Sponsors / Open Collective | Low, spiky | None | **Adopt now** — covers signing/domain costs |
| Paid signed builds, free source (source-available convenience) | Modest | Low; must keep `cargo build` path documented | **[target] adopt at v0.5** if maintainer time is the constraint |
| Pro tier: fleet control + advanced cost analytics | Real | Medium — feature-gating can fork the community | Defer to post-1.0; never gate quota/handover (the loop is the OSS promise) |
| Hosted relay / cloud sync | Real | **High** — breaks local-first, adds credential-adjacent surface | Reject (Fleet is LAN/VPN by design, [PROPOSAL.md:111](../../PROPOSAL.md#L111)) |
| Support contracts for teams | Niche | Low | Opportunistic only; team features are a non-goal ([PROPOSAL.md:45](../../PROPOSAL.md#L45)) |

Crystal's fate is the cautionary data point: a beloved MIT
parallel-session tool deprecated for a commercial successor
([full-competitive-ux-review.md:655-660](../full-competitive-ux-review.md#L655)).
Any monetisation that reads as "the free tool is being closed"
will spend the refugee goodwill §5 recruits. Rule: whatever is
in the OSS build at 1.0 stays in the OSS build.

## 8. GTM & community

### 8.1 Positioning (one sentence, testable)

"Orchestrators run your agents. Switchers swap your account.
Juggler does both, quota-aware." — grounded in the finding that
no orchestrator does account juggling and the juggling layer
(cc-switch, claude-swap, ccusage) is a separate, cruder toolset
([full-competitive-ux-review.md:45-49](../full-competitive-ux-review.md#L45)).
Competitor anchors: [t3.codes](https://t3.codes/) ·
[github.com/winfunc/opcode](https://github.com/winfunc/opcode) ·
[conductor.build](https://www.conductor.build/) ·
[github.com/ryoppippi/ccusage](https://github.com/ryoppippi/ccusage).

### 8.2 Launch sequence

1. **v0.1-beta:** Show HN ("Show HN: AI Juggler — quota-aware
   juggling for Claude Code accounts") leading with the WOW-b
   screenshot — launch-time quota forecast — plus the honest
   fidelity label as the trust hook. Same week: r/ClaudeAI,
   Claude Code Discord, and a post in Vibe/Crystal migration
   threads (§5.1). Submit to awesome-claude-code, awesome-tauri,
   and awesome-rust lists (each is a PR, not an ad).
2. **v0.2:** "Claude + Codex, one quota picture" — target Codex
   community + ccusage users ([github.com/ryoppippi/ccusage](https://github.com/ryoppippi/ccusage))
   with the cost-ledger CSV story.
3. **v0.5/1.0:** migration-guide refresh + a comparison page
   (Juggler vs. Squad/Vibe/T3 on the juggling layer only — never
   claim board/orchestration parity that was not built).

### 8.3 Migration guides (ship with the phases noted)

| From | Guide contents | Ships |
|---|---|---|
| Vibe Kanban | Export projects from SQLite-backed board → import as Juggler projects; map task-attempts to sessions; what does not transfer (card history) stated plainly | v0.1-beta |
| Crystal | Point Juggler at existing worktrees; session list appears via transcript scan; no re-login beyond per-account vendor flow | v0.1-beta |
| Claude Squad | Import `~/.claude-squad/config.json` profiles (`default_program`, named profiles per full review) → Juggler profiles; tmux sessions finish naturally, new ones launch in Juggler | v0.5 (Ph.3 import) |
| cc-switch / aisw | Import named profiles; tray habits mapped ("your one-click switch is now per-session launch") | v0.2 |
| T3 Code | Import t3-managed `ClaudeCodeProfiles` — already the gate-M1 import path ([gate-M1.md:33-35](../../qa/gate-M1.md#L33)) | v0.1-beta (exists) |

Each guide ends with an honest "stay where you are if…" box.
Refugee trust is the asset; a guide that oversells spends it.

### 8.4 Community infrastructure (before Show HN, not after)

GitHub Discussions on; `good-first-adapter` issue label backed
by the Phase 2 conformance kit; a public roadmap mirroring §2;
CHANGELOG discipline per [QUALITY.md:79](../../plan/QUALITY.md#L79).
The adapter guide is the growth engine long-term: T3 ships 6+
harnesses ([full-competitive-ux-review.md:140-143](../full-competitive-ux-review.md#L140));
Juggler will not out-build that, so it must out-enable it.

---

## 9. Risk register

Likelihood/impact are **[judgement]**, reviewed at each gate.
Every risk names an early-warning signal so it is detected, not
discovered.

| # | Risk | L×I | Mitigation (owner: maintainer unless noted) | Early warning |
|---|---|---|---|---|
| R1 | Vendor ToS tightens against multi-account supervision | M×H | Stay inside ADR-0003 guardrails: passive-only, click-executed, no proxy/rotation ([docs/adr/0003-automation-policy-compliance-guardrails.md](../../adr/0003-automation-policy-compliance-guardrails.md#L1)); every feature degrades to single-account value ([PROPOSAL.md:69](../../PROPOSAL.md#L69)); surface vendor terms in-product at account-add | Vendor changelog/ToS diff; adapter `verified_against` drift warnings spike |
| R2 | Harness API/format change breaks an adapter (Claude weekly; Codex 0.134.0 precedent, [PROPOSAL.md:132](../../PROPOSAL.md#L132)) | H×H | Versioned parsers + committed fixture corpus + golden-file tests that fail loudly on drift (M4-1 pattern, [EPICS.md:103](../../plan/EPICS.md#L103)); capability probing at runtime, never assume (SPEC design rule); standing 10–15% drift tax (§3) | Fixture CI failures on a harness update; `verified_against` mismatch banner in dogfood |
| R3 | Credential trust incident (real or perceived) | L×H | Metadata-only login detection precedent ([full-competitive-ux-review.md:323-326](../full-competitive-ux-review.md#L323)); Phase 0 decision D1 makes the import copy explicit, tested, and documented; `guardrails.rs` asserts no credential reads outside the carve-out; SECURITY.md reporting path exercised before beta | Any issue/PR mentioning credentials; guardrail test added *before* the feature it guards |
| R4 | Scope creep into "yet another orchestrator" | H×M | Non-goals enforced in design review ([PROPOSAL.md:136](../../PROPOSAL.md#L136)); review queue gated on unprompted demand (§2, Phase 3); every new screen must name the funnel stage it serves (§6.3) or it is cut | Phase scope growing after entry criteria met; PRs >400 lines ([QUALITY.md:26](../../plan/QUALITY.md#L26)) becoming normal |
| R5 | Handover fidelity failure burns first-user trust | M×H | Fidelity tier displayed *before* confirm; copy the whole session file set and verify parse (Phase 1); never market above the proven tier; fidelity complaint rate is the north-star guardrail (§6.1) | Golden-path fixture with subagents/todos fails; beta fidelity issues >5% of loops |
| R6 | Tiny-team bus factor / maintainer burnout | M×H | Cohorts of 5, not open flood (§5.2); agent runs bounded by RUNBOOK stop conditions ([RUNBOOK.md:32-43](../../plan/RUNBOOK.md#L32)); de-scope levers pre-agreed per phase (§3 table) so cutting is a decision already made | Gate slips twice consecutively; review queue >1 week |
| R7 | Competitor closes the juggling gap first (cc-switch adds sessions; T3 adds named accounts) | M×M | Speed on the loop, not breadth (§1); migration guides + import paths make Juggler the destination even if a rival ships first; conformance kit invites the rival's users to bring their harness with them | Competitor release notes mentioning quota-aware handover (anchors: [github.com/farion1231/cc-switch](https://github.com/farion1231/cc-switch), [t3.codes](https://t3.codes/)) |
| R8 | Signing/notarisation or updater failure at release | L×H | M5-1/M5-2 negative tests in Phase 1/3 gates (§4); fresh-VM install is an exit criterion, not a hope; beta channel before stable ([PROPOSAL.md:150](../../PROPOSAL.md#L150)) | Nightly signed-build job red; Gatekeeper warnings in dogfood |

## 10. Decision log — open questions for the maintainer

Each item has a recommended default. Silence adopts the default
at the Phase 0 gate; none of these block Phase 0 starting.

| # | Question | Recommended default | If rejected |
|---|---|---|---|
| D1 | Credential byte-copy on import vs. SPEC S8 "never copied" ([full-competitive-ux-review.md:331-338](../full-competitive-ux-review.md#L331)) | Amend S8 to the 0700-managed-dir carve-out + guardrail test (spec follows honest code) | Remove the copy; users re-login per imported account (worse onboarding, purer spec) |
| D2 | Gate-M1: close, or waive with named criteria? ([gate-M1.md:54-67](../../qa/gate-M1.md#L54)) | Waive the full-working-session items into the Phase 1 dogfood criterion; record the waiver in the gate file | Phase 1 entry waits on a full manual M1 pass (adds ~1 week, §3) |
| D3 | Fleet control (M7) before or after 1.0? | After 1.0; fleet *link* (read-only) is the 1.0 fleet scope | Pull M7 forward only with a named second maintainer |
| D4 | Review queue: build on inferred demand? | No — read-only diff at v0.5, full queue only after ≥5 unprompted requests (§5/§2 Ph.3) | Build it and accept the R4 scope risk explicitly |
| D5 | Paid signed builds at v0.5? | Yes, convenience-only; source build stays first-class (§7) | Stay sponsors-only; accept slower maintainer sustainability |
| D6 | ZCode adapter depth | Sessions + quota first; handover only after conformance proof | Full adapter — budget the bundle-archaeology tax (§3) |
| D7 | Telemetry: any network event export before 1.0? | No — local store only, opt-in export designed but off until 1.0 | Ship opt-in export at v0.2 with the §6.2 identity rule as a guardrail test |

---

## 11. The 90-day plan (week by week)

Starts the Monday after D1/D2 are answered. Assumes §3 staffing;
each week ends with `main` green ([RUNBOOK.md:21](../../plan/RUNBOOK.md#L21)).

| Wk | Critical path | Ships / proves | Gate |
|---|---|---|---|
| 1 | Phase 0: README/CHANGELOG truth, capability downgrade, honest empty states | Docs-consistency check live | D1/D2 recorded |
| 2 | Gate-M1 disposition; `guardrails.rs` skeleton (writes confinement) | Gate file signed/waived | **P0 gate** |
| 3 | Statusline quota feed (M3-1) behind a feature flag | First Measured quota snapshot in dogfood | — |
| 4 | Usage engine windows + burn-down (M3-2) | Forecast shown in NewSession (internal) | Design checkpoint |
| 5 | Status emission: `QuotaBlocked` + `WaitingUser` end-to-end | `quota_wall_hit` event emittable | — |
| 6 | Stuck heuristics v1 (M2-5 fixtures) | Inbox v1 with real rows | — |
| 7 | Handover: `transcript_ref` fix + source freeze + sibling-file copy | Fidelity fixture (subagents+todos) green | — |
| 8 | Handover moved to `juggler-handover`; PTY-independent golden path | Handover test runs in CI | Design checkpoint (wizard) |
| 9 | Handover wizard: headroom ranking + fidelity label + lineage view | Dogfood loop closed internally | — |
| 10 | Worktree-on-launch + same-branch warning; DB backup/restore | Safety nets complete | — |
| 11 | ⌘K session search; Resume/Relaunch; a11y P1 pass | Daily loop complete | **Internal dogfood week** |
| 12 | Signed/notarised build; fresh-VM install test; migration guides (Vibe/Crystal/T3) | Beta candidate | — |
| 13 | **v0.1-beta to cohort 1 (5 users, §5)**; Show HN draft staged, not posted | First external WJLC counted | **P1 gate review** |

Weeks 14+ (post-90-day, for orientation): beta cohorts 2–3,
Show HN on the first week with ≥10 external WJLC, then Phase 2
entry review against §5.3. If week 13's gate fails, repeat weeks
11–12 with the failure as the scope — do not start Codex on a
loop that has not closed for strangers.

---

## Appendix — deliberate omissions
No re-audit of UI/handover/competitors (deep 01–11 + full review are the evidence); no dates beyond 90 days (§3 ranges are **[target]**, re-estimated per gate); no cloud/account/team tier without amending [PROPOSAL.md:40-47](../../PROPOSAL.md#L40) via the decision log. *Next artefact: the Phase 0 PR, not another plan.*









