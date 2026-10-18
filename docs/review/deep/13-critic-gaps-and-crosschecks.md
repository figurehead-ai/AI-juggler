# 13 — Critic: Gaps, Contradictions & Cross-Checks (Deep)

> Role: completeness critic over parts 01–12. This file does not
> re-review the product; it audits the review. Every claim below
> was checked against the part files on disk (`wc -l`, heading
> scan, spot-reads), not against the parts' self-reported
> summaries. Where this file and a part disagree, the part's
> own text (quoted section) wins until a maintainer rules.

## 0. Verdict in one paragraph

The set is complete in coverage and honest in labelling, but
not yet safe to build from without a deduplication pass: the
same five features (quota/forecast, handover wizard, inbox,
⌘K palette, `usage_events` schema) are specified two to four
times with materially different details, and at least three
direct contradictions (Stop confirmation, Conductor pricing,
migration version ownership) would produce divergent
implementations depending on which part an engineer opens
first. No part is missing or stub-length; the risk is
over-specification, not under-specification.

## 1. Existence & length verification (measured)

Command run by critic: `wc -l docs/review/deep/*.md` plus
`grep -c '^#'` per file. Measured 2026-10-13 session.

| Part | File | Lines (wc) | Headings | >300? |
|---|---|---|---|---|
| 01 | `01-ui-shell-and-rail-button-audit.md` | 413 | 52 | pass |
| 02 | `02-ui-panels-sessions-terminal-button-audit.md` | 426 | 22 | pass |
| 03 | `03-ui-onboarding-handover-settings-audit.md` | 492 | 24 | pass |
| 04 | `04-competitor-t3-and-opcode.md` | 585 | 35 | pass |
| 05 | `05-competitor-claude-squad-and-conductor.md` | 599 | 34 | pass |
| 06 | `06-competitor-vibe-crystal-switchers.md` | 597 | 52 | pass |
| 07 | `07-personas-and-journeys.md` | 452 | 47 | pass |
| 08 | `08-architecture-data-supervisor-security.md` | 600 | 25 | pass* |
| 09 | `09-handover-and-quota-engine-design.md` | 542 | 26 | pass |
| 10 | `10-product-proposal-missing-features.md` | 505 | 38 | pass |
| 11 | `11-ux-redesign-screen-spec.md` | 454 | 48 | pass |
| 12 | `12-roadmap-metrics-risks-gtm.md` | 548 | 36 | pass |

Total: 6,213 lines across 12 parts. `*08` sits exactly on
its 600 ceiling — any future edit must delete as much as it
adds or the part breaches its own target band. Parts 01/02
sit below the 450 floor used by competitor parts; that is
consistent with their narrower UI-audit scope and is not
counted as a gap, but a reader comparing raw lengths should
not mistake 413 lines of control tables for thinner evidence:
01 carries 33 `### B-` headings (see §2).

Self-report cross-check: every part's submitted line count
matches `wc -l` within ±1 (trailing-newline counting). No
part inflated its count. Heading density varies 22–52;
02's low count reflects table-driven controls (SP-/PP- IDs)
rather than missing structure — verified in §2.

## 2. Spot-reads (two parts read in depth, rest sampled)

### 2.1 Part 01 — button-by-button claim: TRUE

`grep '^### B-'` returns 33 headings spanning B-01…B-38
(numbering groups some controls, e.g. B-02 covers four nav
items). Spot-read B-22 (Stop, `WorkPanels.tsx:100`), B-30
(NewSession live), B-32 (Profiles Delete, no confirm/undo),
B-36–B-38 (primitives, TerminalView, dead code). Each entry
carries location, backend command, success/error behaviour,
accessibility note, verdict and redesign — the enumeration
is real, not a heading shell. The dead-code finding
(`NewSessionModal` duplicate, unused `stopSession` export)
is independently repeated by 02, which corroborates it.

### 2.2 Part 02 — same surface, different ID scheme: TRUE

02 audits the same `WorkPanels.tsx` surface with SP-/PP-/NS-
IDs in tables (SP-01…SP-11 spot-read). Coverage is genuine
(9-state lifecycle vs 5 tones, FitAddon-only xterm, ended-
session action vacuum), but the ID scheme and several
redesigns diverge from 01 for identical controls — see
contradiction C1 and dedupe rows D6–D7. A builder cannot
implement both parts literally.

### 2.3 Competitor parts — URL citation claim: TRUE

Measured URL counts: 04 = 85, 05 = 151, 06 = 54 `https://`
links. Spot-reads confirm official GitHub/docs/pricing URLs
attached to competitor claims and `file:line` for Juggler
claims. Caveat: URL presence proves citation, not freshness;
star/funding figures remain snapshot-dated (see §4).

### 2.4 Part 07 — first-person personas claim: TRUE, labelled

Each of Maya/Devon/Priya/Jonas/Sara/Alex carries two
`[composite]` blockquote lines in first person (e.g. Maya:
"I pay for two Max accounts so I never stop…") plus an
italic first-person diary entry. The `[composite]` label and
the header disclaimer ("not interviews — validate §9") are
present on the page, so the verbatim style does not deceive
a careful reader. It will deceive anyone who copy-pastes a
quote without its label — see caveat K4.

### 2.5 Part 10 — acceptance criteria claim: PARTIAL

Every mini-PRD §1…§20 contains an `**Acceptance.**` sentence
(24 matches). Spot-read §1: "Claude ±5% vs `/usage` over a
full window; forecast MAPE ≤30% after 20m burn" — measurable
and falsifiable, good. But no PRD uses Given/When/Then, none
lists negative/edge acceptance (empty, stale, denied), and
several acceptances depend on backends that 09/11 declare
unbuilt. Verdict: acceptance *present*, acceptance *suite*
not yet testable. Roadmap part 12 inherits this gap.

## 3. Direct contradictions between parts

### C1 — Stop: confirm sheet vs explicitly no confirm

01 (B-22) prescribes a confirm sheet: `Stop "{label}"?
Running work will end.` with danger button. 02 (SP-04)
argues the opposite in so many words: "Confirm only if…?
No — Stop is recoverable via Resume, so: no confirm, but
toast with Undo (Resume) for 10s". Both cannot ship.
**Ruling needed:** critic favours 02 (recoverable action +
undo beats modal friction, and 01 itself demands Undo
everywhere else), but the maintainer must strike one text.

### C2 — Conductor pricing: debunked in 05, reused in 07

05 (§7.2) establishes that the parent review's $50/$60
Conductor figures are stale and that exact paid tiers are
unresolved (official pricing page gated). 07 nevertheless
grounds Devon's willingness-to-pay in "vs Conductor Pro
$50 [pricing]" as a factual anchor. 12 largely avoids the
figure. **Fix:** 07's anchor must be reworded to
"employer seat budget" without the debunked price, or
footnoted `[unresolved per 05 §7.2]`.

### C3 — Migration ownership: V5 defined twice

08 (§3.3) proposes v5 creating `usage_events` + `lineage`.
09 (§H7) assigns V5 to lineage/handover and (§Q) V6 to
`usage_events` + `quota_samples`, with different columns
and index names (`idx_usage_session` vs `idx_usage_acct_ts`).
10 (§§1/6) consumes `usage_events` without owning a schema.
An engineer applying both migrations corrupts `user_version`
sequencing. **Canonical:** 09 (it sequences by build order
and says so); 08's §3.3 must be marked superseded.

### C4 — Handover wizard: 3 steps vs 7 steps

01 (B-23) and 02 (SP-08/§11.3) specify a 3-step wizard
(target → fidelity/carry-list → confirm). 03 (§6.3) proposes
a 7-step wizard for the same flow. 11 re-specs the wizard
again at screen level. Step count is not cosmetic: it drives
component structure, telemetry events and test fixtures.
**Canonical:** 11 for screens, 09 for fidelity semantics;
01/02/03 wizard texts should point there, not re-specify.

### C5 — Quota forecast maths stated twice, differently

09 (Part Q) designs EWMA time-to-exhaust with staleness
decay and confidence tiers (Measured/Computed/Estimated).
10 (§1) restates burn as "EWMA tokens/min (30m); forecast =
remaining ÷ burn with widening confidence band" — compatible
in spirit, different in constants and silent on decay.
12 then schedules forecast accuracy targets atop 10's
version. **Canonical:** 09; 10 §1 should cite 09's formulae
instead of paraphrasing them.

### C6 — Minor: status vocabulary drift

02 demands a 9-state chip set with a `Stuck?` honesty suffix;
11 (§2.4) re-specifies status encoding normatively; 01 uses
the same state names with different tone language. Not a
logical contradiction, but three normative tables for one
enum guarantees drift. Canonical: 11 §2.4.

## 4. Claims needing a caveat before external use

- K1. Contrast/focus claims (01, 02, 03, 11): all rest on
  code-read of `styles.css:24` (faint 10.5px) and modal code.
  No browser, VoiceOver or screen-reader pass exists. Any
  WCAG AA assertion must read "code-inferred, unverified".
- K2. Quota percentages in every wireframe (01–03, 10, 11)
  are illustrative until the M3/statusline Measured feed
  exists (SPEC:192). Screenshots of these docs must not be
  shown to users as product capability.
- K3. Competitor magnitudes: T3 funding (no primary source),
  Melty $22M vs $24M (05 flags disagreement), cc-switch
  ~140k stars (06 flags unverified string), Vibe 26.9k /
  Crystal 3.1k snapshots. Safe externally only as
  "order-of-magnitude, snapshot-dated".
- K4. Persona quotes/diaries (07, echoed in 10/12): composite
  verbatim-style, not field interviews. Validation plan
  (dogfood diaries, Priya n=3, Sara security gate, Alex n=5)
  must run before any quote leaves the building.
- K5. Handover fidelity: TRANSPLANT is advertised in code
  (`claude_code.rs:111-120`) while the handover crate is a
  doc-only stub; sibling-file (subagent/file-history) loss,
  dotted/long-cwd mangling and compaction loss are
  fixture-evidenced only. No live Claude 2.1.296 dogfood
  matrix (09 §H8) has run. Do not promise F3+ fidelity.
- K6. Scale numbers (08: 10 accounts / 50 sessions) are
  reasoned estimates from Mutex/rescan/sysinfo code paths;
  no load test exists. Present as hypotheses with the
  measurement plan, not as limits.
- K7. ZCode/Codex semantics: ZCode SQLite session-clone
  schema coupling and Codex exec-resume behaviour are
  unverified against real binaries; Codex quota has no
  sanctioned API, so its percentages stay Estimated until
  user-confirmed denominators calibrate them.
- K8. Inferred demand: present/client mode, budget-cap
  alerts, review/diff queue and smart routing are marked
  `[inferred]` in 07/10/12. They are hypotheses gated on
  dogfood/interviews, not roadmap commitments.
- K9. Negative evidence: "no multi-account/quota in
  Conductor/Squad/Vibe/Crystal/aisw" means docs-and-search
  found none and in-app settings were not inspectable for
  the closed-source tools. Phrase as absence-of-evidence.
- K10. Terminal behaviour: attach-failure and xterm remount
  scroll-loss claims (01, 02) were never run against a live
  Tauri backend. Treat as code-read inferences.

## 5. Gaps (missing, not merely duplicated)

- G1. No part owns the toast/confirm/undo system as a
  buildable spec: 01 §8 and 02 both demand it, 11 defines a
  dialog/focus contract, but props, queueing, timing (10s
  undo appears in both 01 and 02 — at least consistent) and
  telemetry names are nowhere centralised. Needs one owner.
- G2. No accessibility verification plan with dates/owners:
  four parts defer to a "Phase C empirical pass" that 12
  does not schedule as a gate. Deferral without a gate is
  how K1 ships.
- G3. No single settings inventory: 03 audits absence vs
  SPEC §9.2, 10 lists settings per PRD, 11 specs seven tabs.
  Three inventories will diverge on first implementation.
- G4. Error-state copy: parts specify empty/loading states
  well (11 §8 matrix) but backend error strings (handover
  failure taxonomy in 09 H4 excepted) lack verbatim copy in
  01–03 redesigns beyond "toast Retry".
- G5. Fleet/Usage remain ComingSoon in product and are
  correspondingly thin in every part; 10/12 correctly scope
  fleet to schema+teaser, but no part specs the teaser
  screen itself. Acceptable if deliberate — flagged so the
  omission is a decision, not an accident.
- G6. No part reconciles persona set naming across files
  beyond 10's explicit freeze (P1 Maya…P6 Alex matching
  07). 12's recruiting cohorts (Vibe/Crystal refugees) are
  not mapped to personas; GTM-to-persona traceability gap.

## 6. Cross-part feature deduplication map

Read: "Canonical home" is the one place a builder should
implement from; other parts should shrink to a pointer.

| # | Feature (proposed 2–4×) | Appears in | Canonical home |
|---|---|---|---|
| D1 | Quota engine + forecast | 06 S1–S10, 09 Q, 10 §1, 11 Usage | 09 (maths) + 10 §1 (product) |
| D2 | Handover wizard + fidelity tiers | 01 B-23, 02 §11.3, 03 §6.3, 09 H, 11 wizard | 09 H (semantics) + 11 (screens) |
| D3 | Needs-attention inbox | 02, 07 journeys, 10 §2, 11 §4.1 | 11 §4.1 (screen) + 10 §2 (PRD) |
| D4 | ⌘K command palette | 01 §9, 04 steal, 10 §10, 11 palette | 10 §10 + 11 (interaction) |
| D5 | `usage_events` / quota schema | 08 §3.3, 09 H7/Q V5–V6, 10 §§1/6 | 09 V5–V6 only (see C3) |
| D6 | Stop/Delete confirm + undo | 01 B-22/B-32, 02 SP-04 | Ruling per C1, then 11 contract |
| D7 | Toast/banner system | 01 §8, 02 §10, 11 §2.5 | 11 §2.5 (needs G1 props) |
| D8 | Worktree isolation | 05 §3, 06 S9, 10 §4, 12 Phase 1 | 10 §4 |
| D9 | Review / diff queue | 05 steal #1, 10 §11, 12 v0.5 | 10 §11 (dogfood-gated) |
| D10 | Status/state chip encoding | 01, 02 §7, 11 §2.4 | 11 §2.4 (see C6) |
| D11 | Notifications centre/digest | 07, 10 §7, 11 tray/hotkey | 10 §7 |
| D12 | Settings (all tabs) | 03 audit, 10 inventory, 11 §tabs | 11 (screens) + 10 (inventory) |
| D13 | Session search/replay/archive | 02 SP-11, 10 §5, 11 | 10 §5 |
| D14 | Cost ledger / USD estimates | 06 ccusage, 09 Q, 10 §6, 11 Usage | 09 (honesty tiers) + 10 §6 |
| D15 | Onboarding funnel fixes | 03 §§1–5, 11 §5 | 03 (audit) + 11 §5 (targets) |

Net effect if un-deduped: an estimated 25–30% of the
proposal surface exists in two normative versions. The fix
is editorial (replace duplicate specs with pointers), not
new writing — roughly one pass per row above.

## 7. Consolidated unresolved (inherited, de-duplicated)

Runtime verification (browser/screen-reader/Tauri live),
Measured quota backend (M3), status emission for
WaitingUser/Stuck/QuotaBlocked, supervisor input-buffer
signal for unsent-input detection, desktop Focus/Attach
adapter (SPEC:336), live handover dogfood matrix
(Claude 2.1.296 × latest × 5 project shapes), ZCode SQLite
schema proof, Codex denominator calibration, Conductor
exact paid tiers, primary T3 funding source, live GitHub
star counts via API, in-app settings inspection for
closed-source competitors, persona validation interviews,
10/50 load test, and maintainer decisions D1/D2 that start
12's 90-day clock. Each item already has an owner part;
this list exists so none is silently dropped between parts.

## 8. Ship-gate checklist for the review set itself

1. Maintainer rules on C1 (Stop) and strikes the loser text.
2. 07's Conductor $50 anchor fixed per C2.
3. 08 §3.3 marked superseded by 09 V5–V6 per C3.
4. Wizard step-count unified (C4) in 01/02/03 pointers.
5. Dedupe pass per §6 table: duplicates become pointers.
6. K1–K10 caveats copied into any external-facing summary.
7. G1 toast/undo spec and G2 a11y gate get named owners.
8. Only then: 12's Phase 0 exit criteria may be declared.

Until items 1–5 land, the correct citation practice is:
engineers cite the canonical home in §6, and treat every
other part's version of that feature as commentary.

— End of critic part 13. Measured files: 12 parts,
6,213 lines, all >300 lines; spot-reads: 01 (B-22/B-30/
B-32/B-36–38), 02 (SP-01…SP-11), 07 (Maya/Devon/Priya),
10 (§1–§2 acceptance); URL counts measured for 04–06.
