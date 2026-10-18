# AI Juggler — Maximum-Depth Report and Proposal

> Front matter for the 13-part deep review assembled as
> `docs/review/AI-Juggler-Maximum-Depth-Report-and-Proposal.md`.
> Every claim below is grounded in a named deep part; this file
> adds no features of its own. Read the part before citing it
> externally — caveats and evidence levels travel with each part.
> Date of assembly: 2026-10-13. Line counts re-verified at assembly.

## How to read this report (10 minutes, 1 hour, full)

| Reader | Read first | Then | ~Time |
|---|---|---|---|
| Maintainer deciding scope | This file §Executive proposal + §Revised roadmap | 13-critic, 12-roadmap §1/§10 | 10 min |
| Implementer (frontend) | 01 + 02 + 03 audits, then 11 screen spec | 09 quota/handover design | 1 hour |
| Implementer (backend) | 08 architecture, 09 engines | 10 proposal §§1–9 | 1 hour |
| Reviewer challenging claims | 13-critic caveats K1–K10 | Spot-check part's `file:line` cites | any |

### Part map (all line counts via `wc -l`, verified at assembly)

| # | File | Lines | Question it answers |
|---|---|---:|---|
| 01 | `01-ui-shell-and-rail-button-audit.md` | 413 | What does shell/rail chrome do per button? |
| 02 | `02-ui-panels-sessions-terminal-button-audit.md` | 426 | What do panels/sessions/terminal do? |
| 03 | `03-ui-onboarding-handover-settings-audit.md` | 492 | Where do onboarding/handover/settings leak? |
| 04 | `04-competitor-t3-and-opcode.md` | 585 | T3 / Opcode — steal, beat, or avoid? |
| 05 | `05-competitor-claude-squad-and-conductor.md` | 599 | Squad / Conductor — how do they scale? |
| 06 | `06-competitor-vibe-crystal-switchers.md` | 597 | What do switcher/orphan users already expect? |
| 07 | `07-personas-and-journeys.md` | 452 | Who is this for, and where does trust break? |
| 08 | `08-architecture-data-supervisor-security.md` | 600 | Will the backend hold 10 accounts / 50 sessions? |
| 09 | `09-handover-and-quota-engine-design.md` | 542 | How should the two signature engines work? |
| 10 | `10-product-proposal-missing-features.md` | 505 | Which 20 features, in what priority? |
| 11 | `11-ux-redesign-screen-spec.md` | 454 | What exactly should each screen be? |
| 12 | `12-roadmap-metrics-risks-gtm.md` | 548 | What order, what metrics, what risks? |
| 13 | `13-critic-gaps-and-crosschecks.md` | 313 | Where do the parts contradict or overclaim? |

### Evidence language used across parts

- `file:line` (repo path) — code-read fact, not runtime-tested.
- `[NEG-EV]` / negative evidence — feature not found in docs/search;
  absence is not a vendor denial (see 04, 05, 06 appendices).
- `[composite]` — persona quotes/diaries in 07 are verbatim-style
  composites, not field interviews; validation is in 07 §11.
- `[inferred]` / `[target]` / `[judgement]` — demand or durations
  that need dogfood/interviews, flagged where they occur.
- Honesty tiers for quota (09 §Q2): Measured / Computed /
  Estimated — never blur subscription quota with USD cost.

## Executive proposal — top 15 decisions

These compress the parts; wording stays with the source part.

1. **Quota-first (Claude-Measured) before second adapter.**
   SPEC orders quota at M3 after M2 adapters; 12 §1 recommends
   Option B (quota-first, Claude-only) against the promise in
   PROPOSAL. No second adapter buys trust while `percent={0}`.
2. **Relabel handover now; rebuild before marketing.**
   V0 copies one mtime-newest JSONL and drops sibling state
   (09 D1, 03 §6); downgrade `TRANSPLANT` until H8 fixtures pass.
3. **Ship fidelity levels F0–F4 with plain labels (09 §H6).**
   Keep T1–T4 as engine internals; users need "what carries".
4. **Move transplant into `juggler-handover` (09 D3).**
   The crate is doc-only today; `commands.rs` is the wrong home.
5. **Needs-attention inbox as default landing (11 §3, 10 §2).**
   Emit `Stuck/WaitingUser/QuotaBlocked` first (model.rs:188
   modelled, never emitted); the inbox is empty without them.
6. **Suggest, never auto-rotate (09 D6, 12, ADR-0003).**
   Recommend + one click; cswap-style auto-rotation is avoided
   under Juggler's click-bound invariant (05, 06 §8).
7. **Worktree-on-launch (06 S9, 05 F1.1, 10 §4).**
   Session is `cwd`-only today; Squad/T3/Conductor prove
   worktrees are table stakes for parallel work.
8. **No fabricated numbers anywhere (01 F-, 03 §7).**
   Remove hardcoded `percent={0}`, "Machine OK" and Fleet
   placeholders; Usage/Fleet ship honest empty states or teasers.
9. **Destructive guards + undo as a system (01 §0, 02 §0).**
   Stop, Delete profile, and handover chips all fire on one
   click today; toast/confirm/undo must land together. Note the
   C1 tension (13 §3): 01 prefers confirm, 02 prefers undo —
   the maintainer must rule (see 13 ship-gate).
10. **⌘K palette + keyboard system (10 §§10/18, 11 §13).**
    Only `⌘B`/`⌘N` exist; T3/Conductor set the ⌘K bar (04, 05).
11. **Explainable routing score, never silent switching (09 Q6).**
    `0.45 headroom + 0.20 affinity + 0.15 fidelity + 0.10 cost
    + 0.10 calm`, shown to the user before any one-click action.
12. **USD ledger is always Estimated (09 D7, 10 §6).**
    ccusage-style Computed USD is the reference (06 §6);
    subscription quota and USD cost stay separate views.
13. **Trust blockers before charts (10 Finding 3).**
    Account health, backup/restore, and audit log gate adoption
    for Priya/Sara even where their RICE scores rank lower.
14. **Review/diff queue is dogfood-gated, read-only v1 (10 §11).**
    Conductor's diff→PR→merge cockpit is the steal target (05
    §6), but Juggler demand for it is inferred, not validated.
15. **90-day plan starts only after D1/D2 rulings (12 §10–11).**
    Stop-confirm (C1) and canonical migration ruling (C3, 13 §3)
    are maintainer decisions; durations marked `[target]` until
    those rulings land. North star: Weekly Juggling Loops Completed.

## Persona summary (from 07; quotes are composites)

**P1 Maya — Solo Indie Multi-Maxer.** Pays for her own stack
(~$450–620/mo in 07's order-of-magnitude estimate) and runs 4–8
sessions/day across 2–3 repos; her quota wall is a momentum wall.
She pays, reluctantly, for anything that keeps shipping when one
account dies mid-feature and tells her the truth about the cost.

**P2 Devon — Staff Engineer, 10+ Parallel.** Employer-funded,
10–20 sessions/day; his wall is throughput and review capacity.
Waiting sessions are invisible today, so admission ("which needs
me?") matters more than charts; he contributes OSS time, not money.

**P3 Priya — Agency/Consultant, Multi-Client.** 6–14 sessions/day,
$600–1,200/mo pass-through billing; her wall is the budget wall.
Per-client separation and redaction are existential, not cosmetic —
she pays (via clients) for provenance, audit, and renewal artefacts.

**P4 Jonas — OSS Maintainer, Bursty & Broke.** $20–100/mo, 3–10
bursty sessions; a free-tier user who contributes code. Refugee
windows from Vibe (Bloop shutdown 2026-04-10) and Crystal
(deprecated Feb 2026) make him a beta-recruit cohort (12 §5).

**P5 Sara — Enterprise-Locked Developer.** 2–6 approved sessions,
no visible spend, procurement-only payment; she cannot self-pay.
Her gate is security review: redaction, audit, and credential
handling are admission tickets, not features she ranks by RICE.

**P6 Alex — Student/Hobbyist Cost-Optimiser.** $20–38/mo, 1–4
sessions/day, free-only and fast to churn; GLM-credit behaviour
(zcode research) proves the segment. He converts only on honest,
cheap forecasting — never on dashboards.

Cross-persona truth (07 §0): quota-blindness is #1 for 5/6, and
nobody pays for charts — they pay for provenance, ROI, and
renewal artefacts. Three walls (momentum / throughput / budget)
need different UI: time-to-exhaust, per-session burn, sessions-left.

## Competitor verdict summary

| Competitor | Verdict (source part) | Juggler move |
|---|---|---|
| T3 Code | Multi-harness surface, no named accounts (04 §1) | Steal worktree/PR lifecycle + ⌘K; beat on honest quota |
| Opcode (ex-Claudia) | Deep but dormant since Oct 2025, AGPL (04 §§4–7) | Steal checkpoint/MCP ideas, never code |
| Claude Squad | `1 task = 1 tmux + 1 worktree` (05 §3) | Steal atomicity; avoid `-y` autoyes |
| Conductor | Diff→PR→merge cockpit; pricing now free (05 §§6–7) | Steal cockpit pattern; do not envy closed polish |
| Vibe Kanban / Crystal | Orphaned parallel-worktree users (06 §§3–4, 9) | Capture via worktrees/import before quota pitch |
| cc-switch / ccusage / cswap / aisw | Switching stakes S1–S10 (06 §8) | Meet S1–S9; S10 (live-session supervision) is unclaimed |

Headline: no competitor owns named multi-account profiles with
upstream quota (04, 05, 06 agree independently). Juggler's wedge
is intact but time-boxed, because T3/Squad/Conductor already
ship the parallel-work surface Juggler lacks.

## P0 checklist (ship-gate; sources in brackets)

- [ ] Relabel handover `F1 Transcript-only (experimental)`; set
  `TRANSPLANT=false` until H8 fixtures pass [09 D1/D4, 03 §6]
- [ ] Emit `Stuck/WaitingUser/QuotaBlocked` status events [11, 07]
- [ ] Claude-Measured quota feed (statusline `rate_limits`) [06 §2, 09 Q1]
- [ ] Remove fabricated `percent={0}` / "Machine OK" / Fleet fakes [01, 03]
- [ ] Toast + confirm/undo for Stop / Delete / handover [01, 02]
- [ ] Fix PID-reuse adoption risk (journal compares PID only) [08 F2]
- [ ] Worktree-on-launch [10 §4, 05]
- [ ] Settings surface replacing the absent SPEC §9.2 page [03 §7]
- [ ] Delete dead `NewSessionModal` duplicate [01, 02]
- [ ] Rule on C1 (Stop confirm vs undo) and C3 (canonical V5) [13 §3]

## Revised roadmap at-a-glance (from 12 §2; durations are targets)

| Phase | Theme | Entry gate | Exit gate |
|---|---|---|---|
| Phase 0 | Honesty fixes (P0 checklist above) | This report accepted | No fabricated numbers; handover relabelled |
| v0.1-beta | Claude quota + inbox + honest handover | Phase 0 green | Dogfood loops (Maya/Devon cohorts of 5) |
| v0.2 | Worktrees, ⌘K, ledger, reset queue | Beta metrics (12 §6) | Return-on-reset works end-to-end |
| v0.5 | Review queue (read-only), MCP/settings sync | Dogfood demand proven | Priya/Sara gates pass security review |
| v1.0 | Fleet teaser → full, Codex/ZCode depth | H8 dogfood matrix green | 10/50 scale test measured, not estimated |

GTM in one line (12 §8): Show HN + five migration guides, beta
recruited in cohorts of 5 from Vibe/Crystal refugee pools; cloud
relay pricing rejected (12 §7). The 13-week week-by-week plan is
12 §11 — it does not start until maintainer D1/D2 rulings land.
