# Pass 2 — Executive Delta: What the First Report Got Wrong

> Assembly lead synthesis. Canonical input: `docs/expected-worfklow.md`
> (21 lines). First pass under review:
> `docs/review/AI-Juggler-Maximum-Depth-Report-and-Proposal.md`
> (6,739 lines, verified by `wc -l`) plus
> `docs/review/full-competitive-ux-review.md` (1,095 lines, FCUR).
> Pass-2 evidence base, cited by filename throughout:
> `01-expected-vs-researched-gap-audit.md` (502 lines),
> `02-unlimited-bidirectional-switching-engine.md` (567),
> `03-smart-onboarding-and-provider-setup.md` (529),
> `04-daily-indicators-and-right-inspector.md` (505),
> `05-process-trees-zombies-and-cleanup.md` (549),
> `06-external-developer-pain-research.md` (600),
> `07-derived-workflows-and-scenarios.md` (524),
> `08-proposal-delta-and-plan-updates.md` (505), and
> `09-critic-crosschecks.md` (260). Parts 01–09 total 4,541 lines.
> No code was changed by pass 2; every proposal below is a design
> correction gated on the decisions in §6 and the unresolved
> appendix in the assembled second-pass report.

## 0. Verdict in one paragraph

The first report was deep but centred on the wrong product. It made
quota forecasting, one-way handover honesty, and a needs-attention
inbox the headline, while the maintainer's workflow centres
unlimited reversible switching (`expected-worfklow.md:7`), a
single right-hand snapshot for any selection (`:14`), and
ownership of every process an Instance spawns (`:15`–`:18`).
Pass 2 does not discard pass 1; it re-centres it. Switching is a
loop, the inspector is the daily product, and cleanup is a
first-class verified workflow. Quota becomes an input to all
three, not the thesis itself (see `08-proposal-delta-and-plan-updates.md` §1).

## 1. What we got wrong in the first 6,739 lines

### 1.1 Wrong centre of gravity (wrong, not thin)

1. **Quota-first thesis.** Prior thesis (PROPOSAL.md:10; D10 §25
   RICE 600) ranked quota + forecast as Must. Correction in
   `08-proposal-delta-and-plan-updates.md` §1 and
   `01-expected-vs-researched-gap-audit.md` §0: the daily motion
   is switch → see → clean; quota only triggers switching.
2. **One-way handover.** SPEC §7 / Prior-09 modelled one directed
   hop plus a linear `prev_handover` chain (SPEC.md:288,302).
   `02-unlimited-bidirectional-switching-engine.md` §2 replaces
   it with a Switch Graph: Work / Epoch / SwitchEdge, cycles,
   N-hop, fork/merge, and cycle-safe uuid dedupe.
3. **Delete-the-Instances-tab proposal.** Deep 01 B-08 / FCUR
   F2.2 proposed deleting Instances, contradicting the mandate
   at `expected-worfklow.md:11`. `01-expected-vs-researched-gap-audit.md`
   §4 reverses this: build the Instance entity the tab promised.
4. **Zombies scoped out.** Deep 08 F-5 ruled Unix `Z`-state
   orphans out of scope. `05-process-trees-zombies-and-cleanup.md`
   §0 shows the maintainer's zombies are orphaned *live* helpers
   (vite/MCP/watchers) forgotten at root exit (journal.rs:34-48).
5. **Silent wrong-account billing missed.** Prior CUX framed
   wrong-account use as a selection error. `06-external-developer-pain-research.md`
   promotes verified headless billing of the machine-default
   Keychain account (claude-code #72552, #74217) to P0; every
   Instance needs a per-Instance identity assertion.
6. **Worktree collisions understated.** CUX marked collisions
   `[inferred]`. `06-external-developer-pain-research.md` §7
   promotes them to verified P0: silent overwrite, no marker,
   repeated in one sprint. Downgrade all inferred ratings until
   re-evidenced (see `09-critic-crosschecks.md` §3, U-1..U-9).

### 1.2 Under-weighted or missing entirely

7. **Right-section snapshot was Missing.** No Provider panel
   exists (SPEC.md:343); prior split Overview/SessionPanel/inbox
   (deep 11 §3, deep 02 §5). `04-daily-indicators-and-right-inspector.md`
   §5 and `01-expected-vs-researched-gap-audit.md` §3.1 specify
   the Snapshot Panel: Usage / History / Quality / Performance /
   Processes / Actions per selection.
8. **Smart onboarding inference layer.** Shipped wizard is
   Claude-only (`OnboardingWizard.tsx:18`) scanning 4 origins
   (`claude_environment.rs:56`); shell aliases, cc-switch DB,
   installed-CLI gate, and project seeds were never probed.
   `03-smart-onboarding-and-provider-setup.md` §§3–5 add
   Found/Likely/Guessed inference, set-level proposals, and
   six flows (a)–(f) with click budgets.
9. **Process-group ownership does not exist.** Zero
   setpgid/setsid/killpg under `crates/`; bare `Command` spawn
   at supervisor.rs:129-140. The F-9 tree-kill fix therefore
   cannot be made safe by editing `stop()` alone
   (`05-process-trees-zombies-and-cleanup.md` §2). Ledger first.
10. **Derived workflows beyond the noon quota wall.** Prior
    validated only pre-written journeys. `07-derived-workflows-and-scenarios.md`
    derives W1–W11 (night, scale, multi-project, outage, pair
    review) with Auto-passive vs Click-confirmed splits per
    SPEC.md:26, and `01-expected-vs-researched-gap-audit.md` §5
    adds D-W1 switch-back, D-W2 mid-subtask, D-W3 multi-switch/day.
11. **Ownerless items remain.** `09-critic-crosschecks.md` §1.1
    names three: plan/subscription expiry (line 3), the quality
    rollup metric (line 14), and the agent-subtask tree (line
    15). No proposal below ships until each has an owner.

## 2. Top 12 new or corrected proposals

| # | Proposal | Canonical home | Priority |
|---|---|---|---|
| P1 | Switch Graph (Work/Epoch/Edge, 2-parent merge) | `02-…switching-engine.md` §§2,6 | P0 |
| P2 | Switch-back resumes original native session + one delta turn; never copy-back | `02-…switching-engine.md` §6.1 | P0 |
| P3 | Portable context pack (`state.json`) + TRANSPLANT gate | `02-…switching-engine.md` §§4,10 | P0 |
| P4 | Per-pair fidelity matrix (Copy/Translate/Regenerate/Drop) | `02-…switching-engine.md` §3 | P0 |
| P5 | Smart scan + Found/Likely/Guessed proposals, flows (a)–(f) | `03-…provider-setup.md` §§3–6 | P0 |
| P6 | 11-field indicator row I-01..I-11 + 3 density modes | `04-…right-inspector.md` §§3–4 | P0 |
| P7 | Right Inspector: 6 tabs × 5 selections, honest signals | `04-…right-inspector.md` §§5–6 | P0 |
| P8 | Spawn ledger + ProcessIdentity verify-before-kill | `05-…cleanup.md` §§2,9 | P0 |
| P9 | Cleanup Centre + Tier 0–3 opt-in (needs D-C1 ruling) | `05-…cleanup.md` §§9–10 | P0-gated |
| P10 | Per-Instance identity assertion (wrong-account billing) | `06-…pain-research.md` §4 | P0 |
| P11 | Leftover Inventory surface (orphans, ports, hog alerts) | `05-…cleanup.md` §§5–8; `01-…gap-audit.md` §3.2 | P1 |
| P12 | Derived-workflow backlog W1–W11 with falsification plan | `07-…scenarios.md` §13 | P1 |

Demotions agreed in `08-proposal-delta-and-plan-updates.md` §2:
quota-forecast standalone, inbox-as-product, and any
lossless-switch marketing each drop one tier until P1–P4 pass
their §10 fixture matrix. Five NEW PRDs (F-N1 pack, F-N2 cleanup
centre, F-N3 inspector, F-N4 indicators, F-N5 switch-back
divergence) and five UPGRADED proposals are specified there
with persona, controls, backend, acceptance, effort, and RICE.

## 3. Terminology decision (requires maintainer confirmation)

Adopt maintainer language in all user-visible copy, decided once
across `01-expected-vs-researched-gap-audit.md` §4,
`04-daily-indicators-and-right-inspector.md` §1, and
`08-proposal-delta-and-plan-updates.md` §3.4 (3-phase migration):

- **Provider** = `HarnessId` (claude-code / codex / zcode). Never
  show "harness" to users; tooltip names the binary.
- **Account** = `Account` (model.rs:145). Aligned; keep.
- **Instance** = `SessionRecord` + `InstanceKind` (model.rs:389);
  **Session** = the conversation inside it (ADR-0007). The
  shipped collapse (`SessionInfo.ts:3`, `WorkflowRail.tsx:295`)
  is the load-bearing bug to split.
- **Profile** has no maintainer word (model.rs:173): demote to
  "Launch settings" inside the launch sheet, never a rail noun.
  Flag: Codex/cc-switch use `profile` = account — never import
  that meaning. Critic C-1 (`09-critic-crosschecks.md` §2) rules
  copy-only renaming (07 §1) superseded by the 01/08 full rename.

## 4. Revised thesis

> AI Juggler is the switching layer for AI coding work: start on
> any Provider Account, exhaust it, and continue the same work —
> chat, rules, project state — on any other Account or Provider,
> unlimited times in both directions, without manual configuration
> (`expected-worfklow.md:6,9`), while always showing which
> Provider / Account / model is running where and owning every
> process an Instance spawns until it is verifiably gone
> (`expected-worfklow.md:13`–`:18`).

Positioning (from `08-proposal-delta-and-plan-updates.md` §1):
recommend A+B — "switching layer" headline, "verified cleanup"
as the trust proof. Quota visibility supports both; it headlines
neither. Success is measured per PROPOSAL.md:142/144: time to
first switch, A→B→A continuity ≥80%, and zero verified-cleanup
false kills — not dashboard views.

## 5. How to discuss and update the plans

### 5.1 Sixty-minute maintainer agenda

| Min | Item | Decision needed |
|---|---|---|
| 0–5 | This delta §0–§1: re-centring | Accept or amend thesis §4 |
| 5–15 | Terminology §3 (critic C-1) | Full rename (01/08) vs copy-only |
| 15–30 | Switching P1–P4 (`02-…switching-engine.md` §§2–10) | 1-click vs wizard tree §8; TRANSPLANT gate |
| 30–40 | Inspector + indicators P6–P7 | Quality metric owner; plan-expiry owner |
| 40–50 | Cleanup P8–P9 (critic C-2 = D-C1) | Tier 1/2 opt-in vs one-click verified |
| 50–60 | Roadmap + unresolved appendix | Phase 0 scope; dogfood dates |

### 5.2 Document-update order after decisions lock

1. `docs/SPEC.md`: amend automation invariant (§26) only as
   D-C1 rules; rewrite §§3,5,6,7,9,12.1 per
   `08-proposal-delta-and-plan-updates.md` §3. No auto-cleanup
   is designed past the manual flow until D-C1 is ruled.
2. `docs/PROPOSAL.md` §§1,6,9: adopt §4 thesis; move quota to
   supporting input; keep PROPOSAL.md:62-63 click-bound rule.
3. `docs/plan/EPICS.md`: resequence per `08-…plan-updates.md`
   §4 (ledger before tree-kill; pack before TRANSPLANT claims).
4. New ADRs 0008–0012 (terminology, switch graph, ledger,
   cleanup tiers, pack format) per `08-…plan-updates.md` §3.
5. First report stays frozen as pass-1 evidence; corrections
   live only in `docs/review/pass2/`. Do not edit the 6,739
   lines in place — supersede by citation, as 01–09 already do.

### 5.3 Rules for the discussion itself

- Every challenge must name a part filename + section; spot-check
  `06-external-developer-pain-research.md` URLs before citing
  pain claims (critic U-1: Reddit evidence is snippets only).
- `[inferred]` items (W5/W6/W11 in `07-derived-workflows-and-scenarios.md`,
  time-saved figures, Gemini/ZCode parity) cannot enter an epic
  until dogfood, interviews, or a live binary proof falsifies them.
- If D-C1 or the rename depth is not ruled in the meeting, the
  default is the narrower scope: manual verified cleanup only,
  copy-only labels, no Phase 0 start. Record the deferral here.

## 6. What pass 2 deliberately did not do

Pass 2 changed no code, ran no live dogfood, fetched no Reddit
thread bodies, and proved no ZCode/Codex binary behaviour. The
assembled report's Appendix lists every preserved unresolved
item verbatim; until those close, fidelity is capped at F1/F0
(`02-unlimited-bidirectional-switching-engine.md` §10,
SPEC.md:282) and the rendering contracts in parts 04–05 describe
data the upstream does not yet emit (model.rs:188-207).
