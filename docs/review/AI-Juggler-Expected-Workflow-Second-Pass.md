# Pass 2 — Executive Delta: What the First Report Got Wrong

> Assembly lead synthesis. Canonical input: `docs/internal/expected-workflow.md`
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
unlimited reversible switching (`expected-workflow.md:7`), a
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
   at `expected-workflow.md:11`. `01-expected-vs-researched-gap-audit.md`
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
> (`expected-workflow.md:6,9`), while always showing which
> Provider / Account / model is running where and owning every
> process an Instance spawns until it is verifiably gone
> (`expected-workflow.md:13`–`:18`).

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
# Pass 2 — Expected Workflow vs Researched Proposal: Gap Audit

> Second pass. Canonical input: `docs/internal/expected-workflow.md` (21 lines).
> Prior research under audit: `docs/review/AI-Juggler-Maximum-Depth-Report-and-Proposal.md`
> (6,739 lines, 13 deep parts) and `docs/review/full-competitive-ux-review.md`
> (1,095 lines, cited as FCUR). Baseline docs: `docs/PROPOSAL.md:1`,
> `docs/SPEC.md:1`, `docs/plan/EPICS.md:1`. This file cites the prior
> reports by part/section, then extends. It does not restate them.
> Terminology rule for this file: **Provider / Account / Instance**
> per `docs/internal/expected-workflow.md:11`; legacy terms are quoted only
> when citing code or prior text.

## 0. Verdict in one paragraph

Prior research optimised the wrong centre of gravity. It made
quota-first, handover honesty, and the needs-attention inbox the
product (Maximum-Depth front matter, Executive decisions 1–6),
while the maintainer's example workflow centres three things the
prior reports under-weight or contradict: (a) unlimited,
reversible switching as the daily motion, not a one-way handover
(`docs/internal/expected-workflow.md:7`); (b) a single right-hand snapshot
surface for any selected session or provider
(`docs/internal/expected-workflow.md:14`); and (c) lifecycle ownership of
everything an instance spawns, including post-close zombies and
dev services (`docs/internal/expected-workflow.md:15`–`:18`). Most sharply,
prior research proposes deleting the Instances tab
(deep 01 §B-08; FCUR F2.2) — the exact noun the maintainer mandates
at `docs/internal/expected-workflow.md:11`. Pass 2 therefore reframes, not
repeats: switching is a loop, the detail panel is the product, and
process ownership is a first-class workflow.

## 1. How to read the traceability matrix

- **Requirement** quotes or tightly paraphrases one sentence of
  `docs/internal/expected-workflow.md`, with its line number.
- **Prior coverage** names the Maximum-Depth part/section or FCUR
  finding that comes closest; absence is stated as absence.
- **Code/SPEC coverage** gives `file:line` in the repo today.
- **Verdict**: Covered / Partial / Missing / Wrong-emphasis.
  Wrong-emphasis means prior work exists but points the product
  away from the maintainer's workflow.
- **Delta needed** is the smallest correction that aligns research
  with the expected workflow; §6 ranks all deltas.

Requirement IDs R1–R14 are used throughout this file. Blank lines
of the source (2, 4, 8, 10, 12, 16, 19) carry no requirement and
are noted here once so the trace is sentence-complete.

## 2. Sentence-by-sentence traceability matrix

### R1 — Scope is exemplary, not exhaustive (source line 1)

| Field | Evidence |
|---|---|
| Requirement | "This is just one of the expected workflow … there might be others derived from this, competition or other ways." (`docs/internal/expected-workflow.md:1`) |
| Prior coverage | Maximum-Depth front matter frames itself as answering fixed part questions (`docs/review/AI-Juggler-Maximum-Depth-Report-and-Proposal.md:21`); deep 10 §28 validates only three pre-written journeys. No derived-workflow method is proposed. |
| Code/SPEC coverage | `docs/SPEC.md:316` scopes UX to v0.1 screens; `docs/plan/EPICS.md:24` fixes milestones M0–M8 with no discovery track. |
| Verdict | **Wrong-emphasis** — prior research treats the example as a checklist to audit against, not a seed to generate workflows from. |
| Delta needed | Add a derived-workflow backlog (§5 of this file) owned alongside the roadmap; every derived flow gets the same traceability treatment as R2–R14 before it can enter an epic. |

### R2 — Many tools, subscriptions, harnesses, models; quota-expiry tracking (line 3)

| Field | Evidence |
|---|---|
| Requirement | "I often use multiple tools, multiple subscriptions, each one has their own harnesses and models … tracking if one subscription quota is expiring" (`docs/internal/expected-workflow.md:3`). |
| Prior coverage | Strongest prior area: Maximum-Depth Executive decision 1 (quota-first) and deep 09 Part Q (Q1–Q9) design the quota engine; FCUR F6.1 reorders quota before adapters. |
| Code/SPEC coverage | Domain: `crates/juggler-core/src/model.rs:145` (`Account`), `:173` (`Profile`), `:277` (`QuotaWindow`); quota feed specced at `docs/SPEC.md:191`; milestones still order quota at M3 in `docs/SPEC.md:453`. |
| Verdict | **Partial** — covered in depth for Claude, but "expiring" is modelled as window reset, not subscription/plan expiry; multi-model identity per session is absent (see R10). |
| Delta needed | Split two clocks in research and UI: (1) quota-window reset (5h/weekly/credits) and (2) subscription renewal/expiry per Account. Prior reports conflate them under "quota"; the maintainer's sentence names subscription expiry as the tracking object. |

### R3 — Start on one tool; switch on exhaustion to another tool, another subscription, or same tool/different subscription (line 5)

| Field | Evidence |
|---|---|
| Requirement | "Work on a project starting with one tool, and then if I ran out of credits … super easily switch to another tool with different subscription or even same tool, sometimes users have multiple subscriptions from same AI lab." (`docs/internal/expected-workflow.md:5`) |
| Prior coverage | Deep 09 Part H designs T1–T4 tiers; FCUR §Q3 audits handover fidelity (F3.1–F3.5). Three distinct switch kinds in the sentence (cross-tool, cross-subscription, same-tool/multi-subscription) are collapsed into one "handover" verb. |
| Code/SPEC coverage | Tiers defined `docs/SPEC.md:269`; flows `docs/SPEC.md:286`; shipped v0 is a single-file byte-copy (FCUR F3.3, `commands.rs` evidence cited there). |
| Verdict | **Partial** — fidelity is well researched; switch-kind taxonomy and "super easily" (clicks, latency, pre-staging) are not measured anywhere in prior reports. |
| Delta needed | Name three switch kinds as separate product flows with separate success metrics: Switch-Account (same Provider), Switch-Provider (cross-tool), Switch-Instance (resume elsewhere). "Super easily" needs a budget: ≤2 clicks, target pre-staged, no manual path entry — none of which prior acceptance criteria state. |

### R4 — All chat history, context, rules, and guidelines carry over; zero manual configuration (line 6)

| Field | Evidence |
|---|---|
| Requirement | "All the chat history, context, rules and guidelines need to work. I don't want to spend time configuring it out all manually. This is really where the power comes." (`docs/internal/expected-workflow.md:6`) |
| Prior coverage | Deep 09 H2–H5 covers transcript fidelity and redaction; project rules sync appears as one click with diff preview (`docs/SPEC.md:289`, EPICS M4-6). MCP/settings sync is deferred to P2 (deep 10 §14; FCUR F1.9). |
| Code/SPEC coverage | Pack format `docs/SPEC.md:296`; rules conversion `docs/SPEC.md:289`; parser drops `tool_use`/`tool_result` text (FCUR F3.4, `claude_transcript.rs` evidence). |
| Verdict | **Wrong-emphasis** — prior research optimises for *inspectable* transfer (preview, diff, confirm); the maintainer optimises for *invisible* transfer ("need to work"). Every preview step prior adds is friction the expected workflow counts as failure. |
| Delta needed | Reframe default to zero-touch carry (history + rules + guidelines + MCP/settings) with preview as an optional audit view, not a gate. Research must state what "rules and guidelines" includes per Provider (rules files, memory files, system prompts, MCP config) — currently only rules files are specced. |

### R5 — Unlimited back-and-forth switching (line 7)

| Field | Evidence |
|---|---|
| Requirement | "Also I want to be able to do this as many times I want and switching back and forth as needed." (`docs/internal/expected-workflow.md:7`) |
| Prior coverage | Return-on-reset is covered as a single A→B→A hop: `docs/SPEC.md:288`, deep 09 H7 lineage, FCUR F1.5 reset queue. No prior section models N switches, switch cost accumulation, or lineage deeper than two hops as a daily pattern. |
| Code/SPEC coverage | Lineage today is one status flip to `HandedOver` (`crates/juggler-core/src/model.rs:205`); deep 03 §6 notes "no pack, no A→B→A chain" in shipped code. |
| Verdict | **Missing** — the defining adverb of the expected workflow ("as many times", "back and forth") has no design, metric, or test in prior research. |
| Delta needed | Promote lineage from a status flag to a chain entity (see §5, D-W1). Acceptance must include a 5-switch same-day fixture with context-integrity checks at every hop; prior H8 test plan (deep 09 §H8) stops at round-trip. |

### R6 — Manual profiles/configurations are too complicated; the tool absorbs the complexity (line 9)

| Field | Evidence |
|---|---|
| Requirement | "Also these profiles, configurations to accomplish this manually are very complicated, this is why we need this tool to take care of it all." (`docs/internal/expected-workflow.md:9`) |
| Prior coverage | Prior reports expand configuration surface: deep 10 §31 inventories settings for every new knob; FCUR F1.9 adds profile clone/sync checkboxes. Complexity is managed by exposing more of it, carefully. |
| Code/SPEC coverage | `Profile` = account + settings overlay (`crates/juggler-core/src/model.rs:173`); overlay today is model + env only (`crates/juggler-core/src/model.rs:161`). |
| Verdict | **Wrong-emphasis** — prior research treats configuration as a surface to perfect; the maintainer treats it as a burden to remove. The word "profile" itself is part of the burden (see §4). |
| Delta needed | State an anti-configuration budget: a new Account must be launchable with zero user-named settings (defaults derived by scan, R8). Every new knob proposed in deep 10 §31 must name which manual step it deletes, or be cut. |

### R7 — Naming mandate: Providers, Accounts, Instances (line 11, first half)

| Field | Evidence |
|---|---|
| Requirement | "add new provider and account (yes we should call them providers, accounts, and instances of the tool)" (`docs/internal/expected-workflow.md:11`). |
| Prior coverage | Prior research keeps `harness` / `profile` / `session` throughout and proposes *removing* the Instances tab (deep 01 §B-08; FCUR F2.2; deep 11 rail redesign). The maintainer's noun is treated as the problem. |
| Code/SPEC coverage | Code has `HarnessId` (`crates/juggler-core/src/model.rs:49`), `Account` (`:145`), `Profile` (`:173`), `SessionRecord` (`:389`); `InstanceKind` exists only as Terminal/Desktop (`:325`). ADR-0007 (`docs/adr/0007-instance-centric-information-architecture.md:38`) already defines Instance as the running occurrence — code never grew the entity. |
| Verdict | **Wrong-emphasis (direct conflict)** — §4 of this file gives the full rename map and migration cost. |
| Delta needed | Adopt Provider/Account/Instance as user-facing and domain language; demote harness/profile/session to adapter-internal or legacy-alias status per §4. Reverse the "delete Instances" recommendation. |

### R8 — Fully smart automated onboarding: scan, recommend, few options (line 11, second half)

| Field | Evidence |
|---|---|
| Requirement | "They need to have a fully smart and automated onboarding wizard. It can't have thousands of options, it needs to do most of the figuring out on its own, that means scanning directories, existing providers … and making recommendations. It needs to be smart." (`docs/internal/expected-workflow.md:11`) |
| Prior coverage | FCUR F2.6 calls onboarding "genuinely the best-built surface" and recommends freezing it; deep 03 audits leaks (E1–E14) and adds guards. Neither extends it to cross-Provider recommendation or new-Provider onboarding mid-life. |
| Code/SPEC coverage | Scan/import/login/summary flow specced `docs/SPEC.md:409`; ADR-0006 (`docs/adr/0006-smart-onboarding-scan-and-import.md:28`) covers Claude only; wizard auto-opens at zero accounts. |
| Verdict | **Partial** — detection exists for one Provider; "making recommendations" (which Account/Provider to add, which to prefer per project, what to import vs create fresh) has no engine, no ranking, and no acceptance test in prior work. |
| Delta needed | Specify a recommender: scan all known Provider footprints, rank candidate Accounts by recency/plan headroom/project affinity, and present one recommended path with alternatives collapsed. Freeze (F2.6) is the wrong verb; the wizard must learn. |

### R9 — Every session labelled: provider, account, model, quota left (line 13)

| Field | Evidence |
|---|---|
| Requirement | "All the sessions should have icons and labels, which provider it is, account, models currently running. Amount of quota left …" (`docs/internal/expected-workflow.md:13`) |
| Prior coverage | Deep 01 §B-11 and deep 02 §7 redesign status chips; the header Provider chip is static text today (deep 01 §B-06). Quota glance is deferred to a rail footer concept (deep 01 §10 wireframe). No prior spec requires the four-tuple (Provider, Account, model, quota) on every row. |
| Code/SPEC coverage | Session row identity lives in the rail (`WorkflowRail.tsx` per deep 01 §B-11); SPEC screen list `docs/SPEC.md:345` promises quota rings only on Overview, not per-row. Model is stored only as a profile overlay field (`crates/juggler-core/src/model.rs:164`), never as running-model telemetry. |
| Verdict | **Partial** — status labelling is well audited; identity labelling (which Provider/Account/model, with quota) is thinner than the maintainer's "all … should have" bar, and running-model is unmodelled. |
| Delta needed | Make the four-tuple a row invariant with an honesty fallback (`model: unknown` beats omission). Prior chip work (deep 02 §7) should be extended, not replaced: status + identity + quota on one row, text always paired with colour. |

### R10 — Select a session or provider: whole right section becomes stats, actions, switch; snapshot of usage, history, quality, performance (line 14)

| Field | Evidence |
|---|---|
| Requirement | "When you select a specific session or provider, whole right section should come with all kinds of stats, actions to manage, switch … whole goal is to a snapshot of usage, history, quality, performance." (`docs/internal/expected-workflow.md:14`) |
| Prior coverage | Prior IA splits this across three places: Overview (global dashboard, deep 11), SessionPanel (terminal-centric, deep 02 §5), and Accounts cards (deep 02 Profiles audit). Deep 11 §3 makes the needs-attention inbox the default landing instead. No Provider-detail right panel exists in any prior screen spec. |
| Code/SPEC coverage | SPEC instance panel `docs/SPEC.md:347` lists status/account/resources/tokens/actions; SPEC has no Provider-detail screen at all (`docs/SPEC.md:343` screen list). "Quality" appears nowhere as a measured dimension in SPEC or prior quota design (deep 09 Part Q measures tokens/USD only). |
| Verdict | **Missing** — this is the largest single gap. The maintainer describes one contextual snapshot surface; prior research describes a dashboard plus a terminal with chips. "Quality" and "performance" as per-Instance/Provider rollups are unresearched. |
| Delta needed | Specify the Snapshot Panel (§3.1): one right-hand surface parameterised by selection (Instance or Provider or Account), holding stats, actions including Switch, and four snapshot lenses — usage, history, quality, performance. Prior inbox (deep 10 §2) becomes its triage feed, not the home screen. |

### R11 — Per-tool, per-session resource use; subtasks and subprocesses tracked (line 15)

| Field | Evidence |
|---|---|
| Requirement | "For each tool, session, we should show how much resources it is using, cpu, ram, did it spawn any sub tasks, sub processes, and allow to track those." (`docs/internal/expected-workflow.md:15`) |
| Prior coverage | Deep 08 §5–§6 treats the process tree as a supervisor correctness problem (kill, adoption, scale); SPEC §5.2 (`docs/SPEC.md:227`) specs per-session RSS/CPU attribution. Subtasks (agent subagents/tasks inside the conversation) are mentioned only as a fidelity risk for handover (FCUR F3.4). |
| Code/SPEC coverage | Descendant query exists (`crates/juggler-supervisor/src/supervisor.rs:347`); footprint learning `docs/SPEC.md:235`. No subtask entity exists; transcript parser drops tool outcomes (FCUR F3.4). |
| Verdict | **Partial, split in two** — subprocess resources: specced, unbuilt, correctly prioritised as M2. Subtask tracking: **Missing** as a user-facing concept; prior research never asks "what is this session's agent doing right now" as a tree the user can inspect. |
| Delta needed | Two trees, one panel: (1) OS process tree with CPU/RAM per node (SPEC §5.2 as written); (2) agent subtask tree parsed from transcript/tool events, linked to the processes it spawned where correlation exists. Prior FMEA (deep 08 §5) covers tree-kill; it never covers tree-*reading* as a daily workflow. |

### R12 — Crashes leave zombie processes: CLIs, terminals, dev services outlive the session (lines 17–18, first half)

| Field | Evidence |
|---|---|
| Requirement | "Sometimes these tools crash, exit, and their subprocesses, cli commands, terminals and dev local services they started end up running long after the session closed, they end up being zombie processes." (`docs/internal/expected-workflow.md:17`) |
| Prior coverage | Deep 08 F-5 concludes "Our zombies: none … out of scope, document it"; F-9 notes force-kill kills only the direct child so subagents survive a stopped session. The maintainer's scenario — *unmanaged* leftovers (dev servers, terminals the tool opened, processes reparented after a crash) — is explicitly scoped out. |
| Code/SPEC coverage | SPEC's only zombie sentence is re-adoption on app restart (`docs/SPEC.md:224`); supervisor stop ladder `crates/juggler-supervisor/src/supervisor.rs:296` (per deep 08 §5). No orphan inventory, no port/owner attribution, no leftover detection exists. |
| Verdict | **Wrong-emphasis** — prior research answers "do *we* leak zombies?" (no) when the maintainer asks "help me *see and clean* the leftovers my tools leak" (unresearched). These are different products. |
| Delta needed | New capability: Leftover Inventory (§3.2) — after any Instance ends, diff its process/port/terminal footprint against the live system and list survivors with owner, age, and resource cost, including dev services identified by listening port. |

### R13 — Super-easy tracking with automatic and manual cleanups, all verified (line 18, second half)

| Field | Evidence |
|---|---|
| Requirement | "Our tool should have super easy way to track that, offer automatic and manual clean ups, all with verifications …" (`docs/internal/expected-workflow.md:18`) |
| Prior coverage | Deep 08 §8 target architecture adds "tree-kill" to the stop ladder; FCUR and Maximum-Depth guardrails forbid autonomous state-changing actions (automation policy, `docs/SPEC.md:26`). No cleanup workflow, verification step, or automatic-cleanup policy is designed anywhere in prior reports. |
| Code/SPEC coverage | Automation invariant `docs/SPEC.md:26` permits only passive behaviour plus one opt-in same-profile auto-restart; cleanup of leftovers is neither permitted nor forbidden — it is unaddressed. |
| Verdict | **Missing, with a policy collision** — "automatic clean ups" cannot be bolted on; it needs an explicit, narrow exception designed like the existing auto-restart exception (opt-in, per-Instance, same-owner only, verified post-kill), or the maintainer's "automatic" must be reframed as "one-click, pre-verified". Prior research never surfaces this decision. |
| Delta needed | Maintainer decision D-C1 (§6): automatic cleanup scope. Until ruled, design manual verified cleanup (identify → confirm with consequence → kill tree → re-scan verification → report) and treat any automation as a separate ADR amending `docs/SPEC.md:26`. |

### R14 — Research other developer problems beyond these examples (line 20)

| Field | Evidence |
|---|---|
| Requirement | "I need you really to research other ways and solve other problems for developers with similar cases. Think what kind of problems … research forums, github …" (`docs/internal/expected-workflow.md:20`) |
| Prior coverage | Prior personas are explicitly composites, not field research (Maximum-Depth front matter: `[composite]`; deep 07 §11 validation pending). Competitor research is deep (deep 04–06) but product-shaped, not forum/issue-shaped; PROPOSAL pain claims cite tracker issues (`docs/PROPOSAL.md:22`) without a forum ethnography behind the *derived* problems. |
| Code/SPEC coverage | Research notes live in `docs/research/` (four files); no forum/issue-mining method or derived-problem backlog exists in `docs/plan/EPICS.md:1`. |
| Verdict | **Partial** — competitor-derived problems are well covered; community-derived problems (wrong-account billing, lost work on crash, quota anxiety rituals, cleanup toil) are asserted from composites. The maintainer asked for the primary sources. |
| Delta needed | Commission the missing sweep (§5, D-W6): mine vendor trackers and forums for the derived-problem list, quote threads with URLs, and re-validate deep 07 composites against them before any persona is used to rank features. |

## 3. The three under-weighted systems (design deltas)

Prior reports name these areas and then design something else.
This section states the missing design at the same depth prior
research gave quota (deep 09 Part Q) and handover (deep 09
Part H), so the maintainer can rule without a third pass.

### 3.1 Snapshot Panel — the right-hand surface (R9, R10)

**What the maintainer describes.** Selection-driven: click a
session or a provider and "whole right section" becomes stats,
manage/switch actions, and a snapshot of usage, history, quality,
performance (`docs/internal/expected-workflow.md:14`). It is one surface
with a parameter, not four screens.

**What prior research built instead.** Deep 11 makes the
needs-attention inbox the default landing (deep 10 §2, deep 11
§3); deep 02 §5 keeps the session panel terminal-first with
chips; Overview stays a global dashboard (deep 11 screen spec).
Consequence: the most common selection in the app (a running
Instance) opens a terminal when the maintainer expects a
snapshot, and selecting a Provider opens nothing contextual at
all — Accounts is a global card grid (`docs/SPEC.md:346`).

**Pass-2 specification.**

- One component, three parameterisations: Instance snapshot,
  Account snapshot, Provider snapshot. Same lens tabs; content
  scoped by selection. Rail selection drives it; Overview
  demotes to an aggregate of the same lenses.
- Lens 1, Usage: quota windows with honesty tier (Measured /
  Computed / Estimated per `docs/SPEC.md:259`), burn forecast,
  reset countdown, per-Instance token/USD split (deep 09 Q7
  ledger feeds this; it does not get its own screen first).
- Lens 2, History: session timeline for the selection, lineage
  chain (R5), account history per project (ADR-0007 decision 0,
  `docs/adr/0007-instance-centric-information-architecture.md:31`).
- Lens 3, Quality (new — define before building): task outcome
  signals the repo already emits — finished vs failed vs
  quota-blocked ratio, handover success rate, resume success,
  stuck frequency. Quality must never be a model-graded score;
  it is operational reliability, labelled as such.
- Lens 4, Performance: CPU/RAM now and peak per Instance
  (SPEC §5.2, `docs/SPEC.md:231`), footprint percentile,
  admission headroom ("this machine fits N more like this").
- Actions row (always visible): Switch…, Hand over…, Resume /
  Relaunch, Stop, Clean leftovers (§3.2), Open terminal. Prior
  wizard work (deep 02 SP-08) slots in as the Switch… flow;
  nothing is discarded.

**Acceptance.** Selecting any rail row updates the panel in
<300 ms (existing bar, `docs/plan/EPICS.md:66`); every number
carries its honesty tier; a Provider with zero Instances renders
an honest empty snapshot, never the fabricated `percent={0}`
pattern prior research condemns (FCUR F2.4).

### 3.2 Leftover Inventory & verified cleanup (R12, R13)

**What the maintainer describes.** Tools crash; their CLI
children, terminals, and dev servers keep running
(`docs/internal/expected-workflow.md:17`). The tool must track them and
offer automatic and manual cleanup "all with verifications"
(`docs/internal/expected-workflow.md:18`).

**What prior research concluded.** Deep 08 F-5: adopted orphans
are correctly counted dead; piped grandchildren reparented to
init are "out of scope, document it". Deep 08 F-9 is closer —
force-kill kills only the direct child
(`crates/juggler-supervisor/src/supervisor.rs:149` per deep 08
§5) — but the fix proposed is a better stop ladder, i.e. cleaner
exits for sessions Juggler still controls. Neither addresses a
dev server (`vite`, `next dev`, a database) started by an agent
three levels down that survives the session by design.

**Pass-2 specification.**

- **Record at launch:** per-Instance footprint baseline —
  process group, child PIDs, listening ports, terminal IDs —
  extending the journal (`crates/juggler-supervisor/src/journal.rs:151`)
  and descendant walk (`crates/juggler-supervisor/src/supervisor.rs:347`).
- **Detect at end:** when an Instance reaches Finished / Failed
  / Stopped, re-scan: any baseline descendant or port still alive
  becomes a Leftover row (owner Instance, age, CPU/RAM, port,
  command line). Orphans reparented to init are in scope here —
  the opposite ruling from deep 08 F-5.
- **Manual cleanup (ship first):** row action Clean… shows the
  exact tree and consequence, kills tree (TERM → grace → KILL,
  reusing the deep 08 §8 tree-kill design), then re-scans and
  reports verified-gone or still-alive with reason. Verification
  is a re-scan, never an assumption from exit codes.
- **Automatic cleanup (decision-gated):** only under D-C1 (§6).
  Recommended shape, mirroring the sole existing exception at
  `docs/SPEC.md:26`: opt-in per Provider, off by default, only
  for leftovers whose owning Instance is confirmed dead, with
  the same post-kill verification and an undo window where the
  leftover is a resumable service (relaunch command recorded).
- **Never:** kill anything Juggler cannot attribute to an
  Instance baseline. Unattributed processes are listed as
  "not ours — not touched", consistent with the non-goal at
  `docs/PROPOSAL.md:47`.

**Acceptance.** Fixture: fake harness spawns a child server on
an ephemeral port, parent is SIGKILLed; within one scan cycle
the Leftover row appears with the port attributed; Clean…
removes it; verification re-scan shows zero survivors. This
fixture does not exist in prior test plans (deep 08 §5 lists
failure modes; none asserts leftover UX).

### 3.3 Subtask tree — what the agent is doing (R11)

Prior research tracks processes (can it be killed?) but never
tasks (what is it doing?). The maintainer asks both in one
sentence (`docs/internal/expected-workflow.md:15`). Subtasks — subagents,
background tasks, tool calls with duration — live in transcripts
the indexer already reads (`docs/SPEC.md:190`). Pass 2 proposes:
parse subtask open/close events into a tree on the Instance
snapshot (parent session → subtasks → the OS processes each
spawned, where PID correlation exists; otherwise "no process —
in-model work"). This reuses the handover finding that sidechain
lines already live inside the main JSONL (FCUR F3.4) instead of
treating them only as copy-fidelity risk. No new data collection
is required; it is a read model over existing parsing, and it is
the missing half of "allow to track those".

## 4. Terminology proposal: Provider / Account / Instance

Mandate: `docs/internal/expected-workflow.md:11`. Method: adopt the
maintainer's three nouns as the only user-facing and domain
vocabulary; map every existing term exactly once; flag where a
naive rename would break compliance language or stored data.

### 4.1 The three concepts, defined once

| Maintainer term | Definition (proposed canonical) | Absorbs today |
|---|---|---|
| **Provider** | A tool/lab offering whose CLI or app Juggler supervises (Claude Code, Codex, ZCode, future). Owns auth model, quota shape, adapter. | `HarnessId` / "harness" (`crates/juggler-core/src/model.rs:49`); "vendor" in compliance prose (`docs/PROPOSAL.md:60`) |
| **Account** | One subscription/login of one Provider, owned by one person, with an isolated state dir and a plan. Multiple Accounts per Provider is the founding case (`docs/internal/expected-workflow.md:5`). | `Account` (`crates/juggler-core/src/model.rs:145`) — already correct; keep |
| **Instance** | One running or tracked occurrence of a Provider under one Account on one project: a tool instance. Has kind Terminal/Desktop, lifecycle, resources, leftovers. Conversations live inside it across switches. | Launched "session" (`crates/juggler-core/src/model.rs:389`); ADR-0007 Instance (`docs/adr/0007-instance-centric-information-architecture.md:38`) |

Naming conflicts to flag, not smooth over:

1. **Provider vs vendor.** "Vendor" carries legal weight in
   guardrails (ToS, "vendor's own login flow", `docs/SPEC.md:426`).
   Keep *vendor* inside compliance/legal sentences; use *Provider*
   everywhere a user points at a tool. A global find-replace would
   damage the compliance posture prior research rightly protects
   (FCUR §Q4).
2. **Provider vs harness.** "Harness" is precise engineering
   language (the CLI being supervised) and appears in the adapter
   contract (`crates/juggler-adapter/src/lib.rs:1`). Users do not
   say harness; the maintainer says provider. Domain/UI rename;
   adapter-internal retention is acceptable if bridged (see map).
3. **Account vs profile.** Profile (account + settings overlay,
   `crates/juggler-core/src/model.rs:173`) is the "complicated
   manual configuration" the maintainer wants absorbed (R6).
   Profile must stop being a user-facing noun; it becomes an
   internal launch recipe with a derived, rarely-edited default.
4. **Instance vs session.** Deepest conflict. ADR-0007 already
   ruled Instance = occurrence, Session = conversation inside it
   (`docs/adr/0007-instance-centric-information-architecture.md:40`),
   but code stores only `SessionRecord` and the UI renders
   Instances as filtered Sessions (FCUR F2.2). The maintainer's
   "instances of the tool" (`docs/internal/expected-workflow.md:11`)
   matches ADR-0007, not the code. Prior research sided with the
   code; pass 2 sides with the ADR and the maintainer.

### 4.2 Exact rename map

| Layer | Today → Proposed | Notes |
|---|---|---|
| UI nav | Instances tab (delete per deep 01 §B-08) → **Instances (kept, real entity)**; Sessions → **Conversations**; Accounts & Profiles → **Accounts** | Reverses F2.2's merge. Profile leaves nav entirely. |
| UI labels | "harness" chip → **Provider** chip (icon + name); profile name → Account label + model | Four-tuple invariant from R9. |
| Domain (Rust) | `HarnessId` → `ProviderId`; `Account.harness` → `Account.provider`; `SessionRecord` → `InstanceRecord` + new `ConversationId` for the inner conversation; `Profile` → `LaunchRecipe` (internal) | `InstanceKind` (`crates/juggler-core/src/model.rs:325`) already fits; keep. |
| Adapter API | `HarnessAdapter` → `ProviderAdapter` (trait alias for one release) | Crate `juggler-adapter` name may stay; renaming crates is churn without user value. |
| DB | `harness` column → `provider_id`; sessions table → `instances` (+ `conversation_id`); profiles table → `launch_recipes` | Versioned migration (next schema v5, cf. deep 08 §3.3); stored wire strings (`claude-code`) unchanged — IDs are data, not labels. |
| Docs | SPEC §3/§4/§9, PROPOSAL §6, EPICS task names: harness→Provider, profile→Account default/recipe, session(board)→Instance where occurrence is meant | Keep one glossary box mapping legacy terms for contributors; research notes in `docs/research/` stay as historical evidence, unedited. |
| User copy | "Add profile" → **Add account**; "Launch with profile" → **Start on [Account]**; "session board" → **Instances** | Matches maintainer verbs: add provider/account, instances of the tool. |

### 4.3 Migration cost (honest)

- **UI copy + nav: S.** String and tab changes; the real cost is
  building the Instance entity the tab always promised (R10/R11),
  which is feature work, not renaming.
- **Domain rename: M.** `HarnessId`/`SessionRecord` touch core,
  adapters, app commands, and generated TS types; do it with the
  v5 schema migration deep 08 already proposes (§3.3) so users
  pay one migration, not two. Type aliases + serde renames keep
  stored JSON compatible.
- **DB: S–M.** Column/table renames in one migration with
  up/down tests (existing bar, `docs/plan/EPICS.md:44`); wire ID
  values (`claude-code`) do not change, so transcripts and
  lineage references survive.
- **Docs: S.** One glossary ADR (proposed ADR-0014, Terminology)
  plus SPEC/PROPOSAL/EPICS sweeps; prior review files are
  evidence and must not be rewritten.
- **Risk if deferred:** every new adapter (EPICS M2) hardens
  "harness/profile/session" into more code and more user copy.
  The rename is cheapest now, while one Provider is registered.

## 5. Derived workflows implied but not stated

Source line 1 invites derivation (`docs/internal/expected-workflow.md:1`);
line 20 demands it (`docs/internal/expected-workflow.md:20`). Each workflow
below follows from the stated example by changing one variable.
Prior coverage is cited so derivation never double-counts credit.

### D-W1 — Switch back (return to the original Account/Provider)

Implied by "back and forth" (`docs/internal/expected-workflow.md:7`).
Prior covers return-on-reset as a suggestion (`docs/SPEC.md:288`;
FCUR F1.5). Missing: return as a *first-class repeated motion*
with delta priming ("while you were away", `docs/PROPOSAL.md:92`)
verified at hop N, not hop 2, and return across Providers
(B→A where A is a different tool), which T3 replay must support
in both directions. Design lineage as a chain with per-hop
fidelity records (extends deep 09 H7).

### D-W2 — Switch mid-subtask (not at a clean boundary)

Implied by "ran out … I want to be able to super easily switch"
(`docs/internal/expected-workflow.md:5`): exhaustion rarely waits for a
subtask to finish. Prior handover assumes a freezable source
(FCUR F3.3 recommends stop/freeze before copy). Missing: what
carries when a subtask is mid-flight — partial tool state,
running subprocesses (§3.3 tree), uncommitted edits. Needs an
explicit "switch now, reconcile on resume" semantics with the
subtask marked interrupted in the target, never silently dropped.

### D-W3 — Multiple switches per day across one project

Implied by "as many times I want" (`docs/internal/expected-workflow.md:7`)
at daily scale. Prior personas run 4–20 sessions/day (deep 07)
but switches are events, not a rhythm. Missing: switch cost
accounting (time, context loss, quota spent per hop), a same-day
switch timeline on the Project snapshot, and guardrails against
thrash (suggest staying when headroom is nearly equal). Metric:
switches/day and continuity rating per switch join the success
metrics at `docs/PROPOSAL.md:142`, which today test only one
round-trip (≥80% continuity).

### D-W4 — Parallel same-Provider Accounts on one project

Stated at `docs/internal/expected-workflow.md:5` ("multiple subscriptions
from same AI lab") but distinct from switching: two Accounts of
one Provider working one repo simultaneously. Prior worktree
research (deep 10 §4; FCUR F1.1) covers parallel isolation
generically. Missing: same-Provider collision semantics — shared
rules files, transcript-dir separation verified per Account, and
wrong-Account billing guard (PROPOSAL Persona B,
`docs/PROPOSAL.md:52`; FCUR F1.4 affinity) promoted from warning
to launch-time confirmation when project history disagrees.

### D-W5 — Add a Provider or Account mid-project (not at install)

Implied by "add new provider and account" in the onboarding
sentence (`docs/internal/expected-workflow.md:11`) read with daily use:
the second Provider usually arrives when the first hits a wall.
Prior onboarding is install-time (FCUR F2.6; deep 03 E14 notes
the wizard "has no memory of account #1"). Missing: mid-project
add flow that scans (§R8 recommender), imports, and offers
"continue this Instance on the new Account" as its final step —
onboarding ends in a switch, not a summary screen.

### D-W6 — Crash, leftover cleanup, and resumed work (the bad day)

Implied by lines 17–18 read with line 5: the crash that leaves
zombies often *is* the reason to switch. Prior crash recovery
(`docs/SPEC.md:250`) resumes the same profile. Missing combined
flow: Instance crashes → leftovers inventoried (§3.2) → work
hands over to a healthy Account/Provider → leftovers cleaned
with verification → original Instance resumed later or retired.
No prior journey (deep 07 §9, deep 10 §28) chains these steps.

## 6. Top 20 corrections to the prior proposal (ranked)

Ranking key: maintainer-workflow impact first, then trust risk,
then cost of deferral. "Prior anchor" cites the text corrected;
nothing below relitigates items prior research already settled
(quota-first ordering, handover relabelling, fabricated-number
removal — all endorsed by this audit and excluded from the list).

| # | Correction | Prior anchor | Expected-workflow basis |
|---|---|---|---|
| 1 | Keep and build the real **Instance** entity; reverse the Instances-tab deletion. | deep 01 §B-08; FCUR F2.2 | `docs/internal/expected-workflow.md:11` |
| 2 | Adopt **Provider / Account / Instance** per §4 (ADR-0014 + v5 migration); Profile becomes internal LaunchRecipe. | Throughout prior (harness/profile/session) | `docs/internal/expected-workflow.md:11` |
| 3 | Build the **Snapshot Panel** (§3.1) as the primary detail surface for Instance/Account/Provider selection. | deep 11 §3 (inbox as home); deep 02 §5 | `docs/internal/expected-workflow.md:14` |
| 4 | Ship **Leftover Inventory + verified manual cleanup** (§3.2); re-open deep 08 F-5's out-of-scope ruling. | deep 08 F-5, F-9 | `docs/internal/expected-workflow.md:17`–`:18` |
| 5 | Rule **D-C1**: automatic-cleanup scope as a narrow, opt-in exception (or explicitly reframe as one-click verified). | Automation policy `docs/SPEC.md:26` (unaddressed case) | `docs/internal/expected-workflow.md:18` |
| 6 | Model switching as an **unlimited loop**: lineage chain entity, 5-hop acceptance fixture, switches/day metric. | deep 09 H7; `docs/SPEC.md:288` (single return) | `docs/internal/expected-workflow.md:7` |
| 7 | Split switch into **Switch-Account / Switch-Provider / Switch-Instance** with a ≤2-click, pre-staged budget for "super easily". | deep 09 Part H (one handover verb) | `docs/internal/expected-workflow.md:5` |
| 8 | Default handover to **zero-touch carry** (history+rules+guidelines+MCP); preview becomes optional audit, not a gate. | deep 09 H5; EPICS M4-6 diff-gate | `docs/internal/expected-workflow.md:6` |
| 9 | Add **mid-subtask switch** semantics: interrupted subtasks marked and reconciled, never dropped (D-W2). | FCUR F3.3 (freeze assumption) | `docs/internal/expected-workflow.md:5` |
| 10 | Turn onboarding into a **recommender** (scan all Providers, ranked recommendation, mid-project add ending in a switch, D-W5). | FCUR F2.6 (freeze); deep 03 E14 | `docs/internal/expected-workflow.md:11` |
| 11 | Enforce the **row four-tuple**: Provider icon, Account, running model (or honest `unknown`), quota left (R9). | deep 01 §B-06/§B-11 (status-only chips) | `docs/internal/expected-workflow.md:13` |
| 12 | Define **Quality** as operational reliability (finish/fail/blocked, handover/resume success) on the snapshot; ban model-graded scores. | Absent (deep 09 Q measures tokens/USD) | `docs/internal/expected-workflow.md:14` |
| 13 | Ship the **subtask tree** (§3.3) beside the process tree; "track those" covers both. | FCUR F3.4 (fidelity footnote only) | `docs/internal/expected-workflow.md:15` |
| 14 | Split **quota-window reset** from **subscription expiry** as two tracked clocks per Account. | deep 09 Part Q (single quota concept) | `docs/internal/expected-workflow.md:3` |
| 15 | Apply the **anti-configuration budget**: zero user-named settings to launch; each deep 10 §31 knob must name the manual step it deletes. | deep 10 §31; FCUR F1.9 | `docs/internal/expected-workflow.md:9` |
| 16 | Promote **wrong-Account guard** (affinity) to launch-time confirmation for same-Provider parallel work (D-W4). | FCUR F1.4 (warning) | `docs/internal/expected-workflow.md:5` |
| 17 | Run the **forum/issue ethnography** and re-validate deep 07 composites before personas rank features (R14). | deep 07 (`[composite]`, §11 pending) | `docs/internal/expected-workflow.md:20` |
| 18 | Add the **bad-day journey** D-W6 (crash → leftovers → switch → verified cleanup → resume/retire) to dogfood gates. | deep 07 §9; deep 10 §28 (no chained journey) | `docs/internal/expected-workflow.md:17`–`:18` + `:5` |
| 19 | Re-sequence EPICS: Snapshot Panel + Leftover v1 join the Claude loop phase; they outrank second-Provider breadth. | deep 12 roadmap; `docs/plan/EPICS.md:26` | `docs/internal/expected-workflow.md:14`–`:18` |
| 20 | Add **switch-cost accounting** (time/context/quota per hop + anti-thrash suggestion) to Usage (D-W3). | deep 09 Q6 (single recommendation score) | `docs/internal/expected-workflow.md:7` |

**Maintainer decisions requested (blocking, in order):**
D-C1 automatic-cleanup scope (#5); D-T1 terminology ADR-0014
adoption (#2); D-S1 Snapshot Panel as primary surface (#3).
All other corrections proceed under existing guardrails
(`docs/SPEC.md:26`) without new rulings.

*End of Pass 2 audit. Companion evidence: prior reports cited
inline; no prior file was modified by this pass.*
# 02 — Unlimited Bidirectional Switching Engine (Pass 2)

> Second pass. Canonical input: maintainer example workflow
> (`docs/internal/expected-workflow.md:1-20`). Prior work cited, not restated:
> Maximum-Depth Report (`docs/review/AI-Juggler-Maximum-Depth-Report-and-Proposal.md`,
> esp. embedded deep `09-handover-and-quota-engine-design.md`, hereafter
> `Prior-09 §H*`/`§Q*`), Full Competitive & UX Review
> (`docs/review/full-competitive-ux-review.md`, hereafter `FCUR F*`),
> `docs/PROPOSAL.md`, `docs/SPEC.md`, `docs/plan/EPICS.md`.
> Every claim carries `repo/file:line`, a prior-report section, or a URL.
> Status labels: `[now]` shippable on Claude-only code, `[M4]` needs the
> handover epic, `[roadmap]` needs a new adapter. No fidelity above what
> fixtures prove may be marketed (Prior-09 §H9; FCUR F3.2).

## 0. Verdict — what pass 1 under-weighted, got wrong, or missed

The maintainer sentence that defines this product is not "hand over
once" but: switch "as many times I want and switching back and forth
as needed" (`docs/internal/expected-workflow.md:7`), with "all the chat
history, context, rules and guidelines" working
(`docs/internal/expected-workflow.md:6`), across "same tool, sometimes …
multiple subscriptions from same AI lab"
(`docs/internal/expected-workflow.md:5`). Prior work models one directed hop
plus a suggested return; the product needs an unbounded, cyclic,
bidirectional switching engine. Six gaps follow.

| # | Gap | Prior position | This file |
|---|---|---|---|
| G1 | One hop, not a cycle | SPEC return is one click, delta-primed (`docs/SPEC.md:288`); Prior-09 lineage is a linear `prev_handover` chain (`Prior-09 §H7`) | §2 Switch Graph: cycles, N-hop, fork/merge (§2-§6) |
| G2 | Return = copy-back | Return "resumes the original session primed with a delta" (`docs/SPEC.md:288`) — mechanism unspecified; v0 code only copies forward (`crates/juggler-app/src/commands.rs:1088-1194`) | §6: switch-back resumes the *original native session*, never a re-copy; delta is an injected turn, not a transplant |
| G3 | No divergence semantics | Prior-09 F-H-11 names "return divergence" in one table row (`Prior-09 §H4`) with no merge rule | §6.2 three-way reconciliation on message-uuid sets; both-progressed is the *normal* case, not an exception |
| G4 | Per-artifact transfer is coarse | Prior-09 §H2 inventories Claude JSONL contents; SPEC §7.2 step 4 lumps rules/memory/todos/diff into one sync flow (`docs/SPEC.md:289`) | §3 per-pair, per-artifact matrix: Copy / Translate / Regenerate / Drop, with a score per cell |
| G5 | Wizard is the only door | Prior parts specify 3-step vs 7-step wizards and never reconcile them (critic C4, `docs/review/deep/13-critic-gaps-and-crosschecks.md:148-156`) | §8 decision tree: when 1-click is *correct*, when the wizard is mandatory; resolves C4 by making step count data-driven |
| G6 | Cross-tool pairs are one tier | SPEC collapses every cross-harness move to T3 replay (`docs/SPEC.md:273`); ZCode target capped without a path up (`docs/SPEC.md:282`) | §3.3 pair matrix differentiates Claude↔Codex, Claude↔ZCode, Codex↔ZCode and roadmap pairs; §4 pack is the shared substrate that lets a pair *graduate* |

Two prior claims are wrong, not merely thin, and are corrected here:

- **"T1/T2 are effectively indistinguishable from having started on
  the target account"** (`docs/SPEC.md:276`). False for T2 as built:
  v0 copies exactly one file chosen by newest mtime
  (`crates/juggler-app/src/commands.rs:1026-1033,1120-1146`), dropping
  sibling subagent transcripts and file-history that live beside it
  (`docs/research/claude-code.md:40,44`; Prior-09 §H2). Until the
  §10 fixture matrix passes for a pair, that pair's ceiling is the
  §3 score, not "lossless" (Prior-09 §H6 rule (a); FCUR F3.4).
- **"Running sessions are never switched underneath the user"**
  (`docs/SPEC.md:286`) is used to justify switch = *new* session only.
  The maintainer workflow switches the *work*, mid-task, when quota
  dies mid-flight (`docs/internal/expected-workflow.md:5`;
  `docs/PROPOSAL.md:23`). The engine therefore needs a defined
  mid-task freeze protocol (§5.2); "start the next session elsewhere"
  does not satisfy the example workflow.

## 1. Terminology — maintainer language mapped to the domain

Maintainer: "we should call them providers, accounts, and instances
of the tool" (`docs/internal/expected-workflow.md:11`). Adopt that language in
all user-visible copy. Internal types keep their names; the mapping
below is normative for UI strings, warnings, and this file.

| Maintainer term | Meaning for the user | Existing domain term (repo) | Conflict flag |
|---|---|---|---|
| **Provider** | The lab + tool family you subscribe to (Claude, Codex, ZCode; later Gemini, OpenCode) | `HarnessId` — `"claude-code" \| "codex" \| "zcode"` (`crates/juggler-core/src/model.rs:47-50`; `docs/SPEC.md:91`) | ⚠ `Profile` is *not* a provider. Vendor code also uses "provider" for model endpoints (ZCode `provider_config.json`, `docs/research/zcode.md:58`); UI must say "Provider (tool)" on first use per screen |
| **Account** | One subscription/login under a provider (e.g. two Claude Max subs) | `Account { harness, label, plan, state_dir }` (`crates/juggler-core/src/model.rs:142-157`) | ⚠ SPEC §3 sketch and code agree; PROPOSAL §6.1 calls the launchable unit a "profile" (`docs/PROPOSAL.md:75`) — user copy must never expose "profile" as a synonym for account |
| **Instance** (tool instance) | One running occurrence of a tool under one account on one project | `SessionRecord` + `InstanceKind::{Terminal,Desktop}` (`crates/juggler-core/src/model.rs:323-355,386-416`); ADR-0007 decision 1 (`docs/adr/0007-instance-centric-information-architecture.md:38-43`) | ⚠ Code names the instance record `SessionRecord`/`SessionId`; ADR-0007 already flags this drift. Keep `Session*` in Rust, render "Instance" in UI; a *session* in UI = the conversation inside an instance (ADR-0007:40-43) |
| **Profile** (internal only) | Account + settings overlay (model, env, MCP) used at launch | `Profile { account, settings_overlay, default_cwd }` (`crates/juggler-core/src/model.rs:170-184`) | ⚠ Highest-risk homonym: cc-switch and Codex both use "profile" for the *account* unit (Codex profiles are config files, `docs/research/codex.md:19-20`). Juggler UI: rename to "Launch settings" wherever user-facing; keep `Profile` in code/APIs |
| **Switch** (new verb) | Moving the *work* (conversation + project state) from one instance to another, any direction, any number of times | No type exists. v0 `session_handover` is Claude→Claude only (`crates/juggler-app/src/commands.rs:1062-1194`); `juggler-handover` is a doc-only stub (`crates/juggler-handover/src/lib.rs:1-13`) | New domain noun required (§2): `SwitchEdge`. "Handover" survives as the *one-hop user action*; "switching" is the unbounded graph behaviour the maintainer asked for |
| **Switch-back** (new verb) | Returning work to a previous instance/account, resuming *its own* native session | SPEC "Return / Resume lineage" (`docs/SPEC.md:288`); no code | Must not be implemented as a second forward copy (§6) |

Naming rule (must): every warning, ranking row, and lineage label in
§7-§9 uses Provider/Account/Instance. Any string that says "profile"
to mean an account, or "session" to mean a running tool, fails review.
This extends ADR-0007's instance-centric IA
(`docs/adr/0007-instance-centric-information-architecture.md:29-58`)
from navigation to the switching vocabulary itself.

## 2. The Switch Graph — the core model prior work lacks

SPEC lineage is a chain: "pack lineage (A→B→A chains)"
(`docs/SPEC.md:302`) and Prior-09 stores `prev_handover`
(`Prior-09 §H7`). A chain cannot represent what unlimited switching
actually produces: A→B→A→C→B, a fork where both A and B kept working,
or a return after three hops. Chains force the second return to
pretend the first never happened. Replace the chain with a graph.

### 2.1 Nodes, edges, and the work identity

- **Work item** (new): the user's continuing task on one project.
  Anchored to `ProjectRecord`, the juggling anchor keyed by canonical
  root (`crates/juggler-core/src/model.rs:367-384`;
  `docs/adr/0007-instance-centric-information-architecture.md:31-37`).
  A work item survives every switch; instances come and go under it.
- **Node** = one *instance-epoch*: an instance (`SessionRecord`,
  `crates/juggler-core/src/model.rs:389-416`) running under one
  account from launch/resume until it stops, is frozen, or hands off.
  Resuming the same native session after switch-back creates a new
  epoch node pointing at the *same* `native_id` — that pointer is
  what makes switch-back a resume, not a copy (§6).
- **Edge** (`SwitchEdge`, new): one executed switch. Fields:
  `id, work_id, from_epoch, to_epoch (null until launch succeeds),
  from_account, to_account, from_provider, to_provider, mechanism
  (native-resume|transplant|replay|pack), pack_id (§4), fidelity_shown
  (§3.4), artifact_manifest_sha256, state (§9), created_at, actor`.
  This generalises Prior-09's `handovers` table (`Prior-09 §H7`):
  keep its columns, drop the assumption that `prev_handover` is the
  only parent — an edge may have **two** parents after a merge (§6.2).
- **Invariant S-G1:** edges are append-only; a failed or undone switch
  adds an edge/state, never rewrites history (extends Prior-09 §H7
  "not history rewriting"). Undo = a compensating resume edge.
- **Invariant S-G2:** the graph is per-project. Cross-project switches
  are exports (T4 pack, `docs/SPEC.md:274`), not edges, because `cwd`
  is baked into every native format (Claude mangled project dir,
  `docs/research/claude-code.md:40`; Codex `session_meta.cwd` absolute,
  `docs/research/codex.md:41,46`).

### 2.2 What "unlimited" formally requires

1. **Idempotent re-entry:** switching A→B→A must not duplicate
   messages. Claude dedupe is first-wins by message `uuid`
   (`crates/juggler-adapters/src/claude_transcript.rs` per Prior-09
   §H7.1); every mechanism in §3 must state its dedupe key (uuid set,
   rollout `session_id`, ZCode `sess_*` id) or it is not cycle-safe.
2. **No hop-count state:** no field may assume depth ≤ 2. Token cost,
   however, *does* grow with hops for replay (§7) — the graph records
   cumulative replay tokens per work item so the UI can say so.
3. **Any node can be a source again** if its native session still
   exists on disk, including `HandedOver` nodes
   (`crates/juggler-core/src/model.rs:205-207,238`). `HandedOver` is a
   *resting* state, not a terminal one — prior UI treated ended rows
   as action-less (FCUR F2.3); the graph requires Resume on every node.
4. **Fork is legal, merge is explicit:** two live epochs on one work
   item (user kept A running while B worked) is a fork node pair.
   The engine never auto-merges (§6.2); it *detects* and asks.

### 2.3 Storage delta (migration after Prior-09 V5/V6)

Critic C3 already rules Prior-09 owns migration numbering
(`docs/review/deep/13-critic-gaps-and-crosschecks.md:137-146`), so
this file does not re-specify V5/V6. Required *additions*, sequenced
after them: `work_items(id, project, title, created_at)`,
`epochs(id, work_id, session_ref, native_id, account, provider,
resumed_from_epoch)`, `switch_edges(… §2.1 …, parent_edge_a,
parent_edge_b)`, `artifact_manifests(edge, rel_path, sha256, bytes,
kind, disposition)`. `sessions.project` already exists
(`crates/juggler-core/src/model.rs:401-403`) as the work anchor key.
Acceptance: an A→B→A→C fixture chain renders as one graph with 4
epoch nodes and 3 edges, and re-opening it after restart is identical
(storage is WAL SQLite, `docs/SPEC.md:22`).

## 3. Switch matrix — per pair, per artifact

Dispositions: **Copy** = byte/move of native artifact, no semantic
change. **Translate** = format conversion with a defined mapping.
**Regenerate** = target rebuilds it (replay turn, re-derived summary,
reconnected MCP). **Drop** = does not carry; must appear on the
carry-list (Prior-09 §H5). "Translate" without a written mapping in
the adapter is marketing, not engineering (Prior-09 §H1, H-1).

### 3.1 Artifact inventory (what can even be asked to transfer)

| Artifact | Native home (evidence) | Notes |
|---|---|---|
| Chat transcript | Claude `projects/<mangled>/<uuid>.jsonl` (`docs/research/claude-code.md:40-41`); Codex `sessions/**/rollout-*.jsonl` (`docs/research/codex.md:39-42`); ZCode SQLite `session/message` (`docs/research/zcode.md:63`) | The conversation itself |
| Context summary / compaction state | Claude `compact_boundary` lines (`docs/research/claude-code.md:45`; Prior-09 §H2.1) | Post-compact live context ≠ raw file |
| Rules & guidelines | `CLAUDE.md` / `AGENTS.md` / `.zcode/AGENTS.md` / `GEMINI.md` (`docs/SPEC.md:289`; `docs/research/landscape-and-stack.md:45-48`) | AGENTS.md is the interchange form |
| MCP configuration | Claude user MCP in `~/.claude.json`, project `.mcp.json` (`docs/research/claude-code.md:62`); Codex `[mcp_servers]` (`docs/research/codex.md:18`); ZCode `mcp.servers` (`docs/research/zcode.md:26`) | Config translates; live MCP *processes* never transfer (Prior-09 §H2) |
| Skills / commands / agents | `~/.claude/skills|agents|commands` (`docs/research/claude-code.md:63`); ZCode `~/.zcode/skills` (`docs/research/zcode.md:26`); Codex plugins/agents (`docs/research/codex.md:18`) | Mostly files; formats differ |
| In-session todos / plan | Unverified inside Claude JSONL (Prior-09 §H2); ZCode has a `todo` table (`docs/research/zcode.md:63`) | Treat as Drop until a fixture proves otherwise |
| Git state & worktree | Session stores `cwd`/branch (`docs/research/claude-code.md:41`); parallel work needs worktrees (FCUR F1.1; `docs/research/landscape-and-stack.md:56-60`) | Shared via the checkout, not the transcript |
| Uncommitted diff | Working tree on disk; SPEC pack carries `diffs/` (`docs/SPEC.md:303`) | Belongs to the *checkout*, so it follows the project, not the account — unless worktrees diverge |
| Terminal env & settings overlay | `SettingsOverlay { model, env }` (`crates/juggler-core/src/model.rs:159-168`); per-account env isolation (`docs/SPEC.md:128-136`) | Model names do not translate 1:1 across providers |
| Subagent state | Inline `isSidechain` lines carry; sibling subagent transcripts do not (Prior-09 §H2; FCUR F3.4) | Live subagents/processes: always Drop |
| Credentials | Keychain/`auth.json`/`credentials.json` per account (`docs/SPEC.md:310`) | **Never** in any pack, edge, or manifest (guardrail, `docs/PROPOSAL.md:61-68`) |

### 3.2 Same provider, different account (incl. same lab, multi-sub)

The maintainer's most common case: "multiple subscriptions from
same AI lab" (`docs/internal/expected-workflow.md:5`). Mechanism: transplant
(SPEC T2, `docs/SPEC.md:272`) — same binary both sides, so format
compatibility is structural, not hoped-for (`docs/SPEC.md:281`).

| Pair | Chat | Summary | Rules | MCP cfg | Skills | Todos | Git/diff | Env/model | Subagents |
|---|---|---|---|---|---|---|---|---|---|
| Claude A→Claude B `[now, unproven]` | Copy* | Copy (raw) | Copy (same files) | Copy (per-dir config, `docs/research/claude-code.md:62`) | Copy | Drop? | Shared checkout | Copy overlay; model names identical | Inline Copy; siblings Copy* |
| Codex A→Codex B `[M4]` | Copy rollout (`docs/SPEC.md:281`) | Regenerate (no compact scar format verified) | Copy (`AGENTS.md`) | Copy (`config.toml` families, `docs/research/codex.md:18`) | Copy plugins | Drop? | Shared checkout | Copy; profile files per 0.134 semantics (`docs/research/codex.md:19`) | Drop (thread children unverified) |
| ZCode A→ZCode B `[roadmap]` | Blocked: SQLite clone, schema-coupled (`docs/SPEC.md:282`; tables `docs/research/zcode.md:63`) | Blocked | Copy | Copy JSON (`docs/research/zcode.md:59`) | Copy | Copy* if clone lands (`todo` table exists) | Shared checkout | Copy | Drop |

`*` = only after Prior-09 §H8 fixtures pass: sibling-file set
(H-F1), torn-tail freeze (H-F2), collision fork (H-F3). As shipped,
v0 is single-file/newest-mtime (`crates/juggler-app/src/commands.rs:1120-1146`)
and its honest label is Prior-09 D1, `F1 Transcript-only
(experimental)` (`Prior-09 §0`), regardless of the `TRANSPLANT` bit
currently advertised (`crates/juggler-adapters/src/claude_code.rs:111-120`).
Target-dir resolution must discover-don't-compute (Prior-09 §H2.2,
H-3): naive `replace('/','-')` at `commands.rs:1121-1122` collides
on dotted/dashed paths (`docs/research/claude-code.md:40-41`).

### 3.3 Different provider (cross-tool), by pair

Mechanism: replay (SPEC T3, `docs/SPEC.md:273`) via the §4 pack, or
pack-only (T4, `docs/SPEC.md:274`) where no priming path exists.
Vendor precedent that replay is legitimate: Codex ships `/import`
of Claude Code/Cursor chats (`docs/research/codex.md:63`;
`docs/SPEC.md:276`). No production Claude↔Codex↔ZCode transcript
converter exists anywhere (landscape finding,
`docs/research/landscape-and-stack.md:54`) — Juggler's converter *is*
the product; each mapping below must land as an adapter function
with golden tests (EPICS M4-1/M4-4, `docs/plan/EPICS.md:103-106`).

| Pair (both directions unless noted) | Chat | Rules | MCP cfg | Skills | Todos | Env/model | Ceiling today |
|---|---|---|---|---|---|---|---|
| Claude ↔ Codex `[M4]` | Translate→Replay: IR turns + distilled tool outcomes (parser drops raw tool text today, Prior-09 §H2) | Translate: `CLAUDE.md`↔`AGENTS.md`; Claude reads AGENTS.md natively ≥v2.1.277 (`docs/SPEC.md:289`; `docs/research/landscape-and-stack.md:48`) | Translate: `.mcp.json`↔`[mcp_servers]` key mapping; unmapped keys listed, never silently dropped | Translate where format maps; else list as Drop | Drop→Regenerate as opening todo turn | Translate by *intent* (reasoning effort tier), never by name string | F1 (Prior-09 §H6) |
| Claude ↔ ZCode `[M4/roadmap]` | Replay from JSONL / from SQLite reads (`docs/research/zcode.md:76-77`); ZCode *target* has no verified prime path — pack-only until proven | Translate via AGENTS.md (`.zcode/AGENTS.md`, `docs/research/zcode.md:26`) | Translate JSON↔JSON (`docs/research/zcode.md:26,59`) | Copy files where dirs align (`docs/research/zcode.md:26`) | ZCode `todo` table → opening turn; reverse Drop | Translate by intent | F1 source / F0 target |
| Codex ↔ ZCode `[roadmap]` | Replay both ways via IR; neither side has a native import for the other | AGENTS.md both sides — Copy | Translate TOML↔JSON | List/Drop | Same as above | Translate by intent | F1/F0 as above |
| Any → Gemini CLI / OpenCode / future `[roadmap]` | Pack-only (F0) until an adapter passes §10; GEMINI.md joins the rules ring (`docs/research/landscape-and-stack.md:48`); these harnesses already run under peer orchestrators (`docs/research/landscape-and-stack.md:11,15`) | Translate via AGENTS.md ring | List/Drop | List/Drop | Drop | List | F0; SPEC adapter-first rule (`docs/SPEC.md:140`; `docs/PROPOSAL.md:35`) forbids promising more |

Direction matters and prior work hides it: "Claude→ZCode" and
"ZCode→Claude" have *different* ceilings (source readability vs
target primability). Every matrix cell, ranking row, and warning in
§7-§8 is therefore computed per **ordered pair**, never per provider
pair. This corrects SPEC §7.1, whose table is direction-blind
(`docs/SPEC.md:269-274`).

### 3.4 Fidelity score — per artifact, then capped composite

Prior-09 §H6 gives levels F0-F4 with user labels; keep the labels,
add the arithmetic the wizard and ranking (§7) need. Per artifact:
Copy=1.0, Translate=0.8 (0.6 if any key/field is listed-dropped),
Regenerate=0.5, Drop=0. Composite = weighted mean (chat .35,
rules .15, git/diff .15, todos .10, MCP .10, skills .05, env .05,
subagents .05), then **capped**: any Drop in chat/rules/git caps at
F1; a `compact_boundary` in the source caps at F2 (Prior-09 §H2.1);
an unproven pair (§10 cell not green) caps at F1 however it is
built. Map: ≥.95→F4/F3 label per Prior-09 §H6 ("Continues exactly"
only for same-account native resume), .75-.94→F3/F2, .45-.74→F1,
<.45→F0. The badge always ships with the carry-list (Prior-09 §H6
rule (a)); the number is internal, the label + list are the UI.
Worked example: Claude A→B done *properly* (siblings copied) scores
≈.93 → F3 "Same conversation, new account". As v0 ships (one file),
siblings Drop → ≈.86 but capped F1 until H-F1 passes — the cap, not
the mean, is the honesty mechanism (FCUR F3.4; critic K5,
`docs/review/deep/13-critic-gaps-and-crosschecks.md:196-201`).

## 4. Canonical Portable Context Pack — exact fields

SPEC §7.3 sketches a directory (`docs/SPEC.md:292-306`); Prior-09
§H5 makes it the preview artifact. Neither fixes the schema, so two
implementations would diverge. Fix it here. The pack is (a) the T4
export, (b) the staging artifact for *every* edge (even transplant
generates a manifest-only pack), and (c) the merge substrate (§6.2).

```
pack/  (format_version: 1)
├── PACK.md            # human brief: task, constraints, decisions, open todos, next steps (docs/SPEC.md:298)
├── conversation.md    # T3: full replay text; T4: distilled summary + last N turns (docs/SPEC.md:299-300)
├── state.json         # schema below — the canonical record
├── manifest.json      # artifact inventory: [{rel_path, sha256, bytes, kind, disposition}] (cf. Prior-09 §H7 session_files)
├── diffs/             # uncommitted diff + per-file patches, or EMPTY marker file (docs/SPEC.md:303)
└── rules/             # AGENTS.md canonical + per-provider renderings actually written
```

`state.json` fields (all required unless marked opt): `format_version`,
`pack_id` (uuid), `created_at` (epoch-ms, `crates/juggler-core/src/model.rs:74-88`),
`work_id`, `edge_id` (opt for standalone export), `source{provider,
account_label, profile_name, native_session_id, harness_version,
epoch_id}`, `target_hint{provider, account_label}` (opt),
`project{root, project_id}`, `git{branch, head_sha, dirty:bool,
diff_stat, worktree_path(opt)}`, `conversation{turn_count,
message_uuids_sha256, compacted:bool, first_turn_at, last_turn_at,
replay_tokens_est}`, `artifacts{rules:[…], mcp:[…], skills:[…],
todos:[…], drops:[{artifact, reason}]}` — every Drop from §3 appears
here, `fidelity{level, score, caps_applied:[…]}`,
`quota_at_creation{account, used_pct, confidence}` (confidence per
`crates/juggler-core/src/model.rs:286-297`), `lineage{parent_pack_ids:[…],
parent_edge_ids:[…]}` (a list — merges have two parents, §2.1),
`redactions:[{kind, count}]` (values never stored; Prior-09 §H5.4).
Unknown fields must be ignored by readers (format churn is the norm:
Claude ships weekly, `docs/PROPOSAL.md:132`; Codex broke profiles at
0.134.0, `docs/research/codex.md:19`). Generation is deterministic
from the Transcript IR; LLM "smart distill" stays post-v1
(`docs/SPEC.md:306`) and, when it lands, writes a *separate*
`summary_llm.md` so the deterministic record is never overwritten.

## 5. Switch execution — freeze, stage, verify, launch

### 5.1 Pipeline (every edge, every pair)

`Resolve → Preflight → Freeze → Snapshot → Stage → Verify → Launch
→ Confirm → Record` — generalises Prior-09 §H7's
`Staged → Verified → Launched → Confirmed`. Resolve uses the
session's own `transcript_ref`
(`crates/juggler-core/src/model.rs:314-321,415`), never a directory
guess (Prior-09 F-H-01). Preflight computes, in order: ordered-pair
mechanism + §3.4 score, compaction scan (Prior-09 §H2.1), target
quota gate (Prior-09 §Q6 hard filters), version-drift parse check
(Prior-09 F-H-06), git/worktree state (§5.3), and the §8 door
(1-click vs wizard). Any preflight failure stops *before* any byte
moves; the failure taxonomy and copy live in §9.

### 5.2 Mid-task freeze protocol (new — the quota-wall moment)

The defining switch happens while the source is mid-turn
(`docs/internal/expected-workflow.md:5`; in-flight work killed by walls,
`docs/PROPOSAL.md:23`). Protocol: (1) stop accepting the fiction
that a live, appending transcript can be copied — v0's raw
`std::fs::copy` (`crates/juggler-app/src/commands.rs:1143-1146`) can
tear the last line (Prior-09 F-H-02); (2) offer exactly two freezes:
**Finish this turn, then switch** (default; supervisor waits for the
turn boundary signal) or **Stop now and switch** (stop ladder,
`docs/SPEC.md:225`; partial turn is marked truncated in the pack,
never presented as complete); (3) snapshot = copy-then-parse-verify
the tail (Prior-09 H-F2). There is no third option "copy live and
hope", and SIGSTOP-style pausing is forbidden (SPEC already bans it
for corruption risk, `docs/SPEC.md:225`). Adapter trait gap: the
contract today has no export/prime/transplant methods at all
(`crates/juggler-adapter/src/adapter.rs:26-124` vs sketch
`docs/SPEC.md:173-177`); adopt Prior-09 §H1's optional
`HandoverSupport` sub-trait verbatim — this file adds no rival trait
(critic dedupe rule, `docs/review/deep/13-critic-gaps-and-crosschecks.md:251-277`).

### 5.3 Git, worktree, and uncommitted work

Git state follows the *checkout*, so same-checkout switches need no
transfer — but two live instances on one checkout on one branch is
the P0 hazard FCUR F1.1 names. Rules: same branch + both live →
warning + offer worktree for the target (worktree-per-instance is
the proven peer pattern, `docs/research/landscape-and-stack.md:56-60`);
uncommitted diff is snapshotted into `diffs/` at freeze (so a later
conflict has a base, §6.2) but *not* re-applied elsewhere — the
target opens the same tree; diverged worktrees → the switch carries
a named patch, applied only from the diff preview (Prior-09 §H5.3;
SPEC diff-preview rule, `docs/SPEC.md:289`). Zombie children of a
frozen instance (expected-workflow pain,
`docs/internal/expected-workflow.md:17-18`) are listed at freeze; cleanup is a
separate confirmed action, never a silent side-effect of switching.

## 6. Switch-back and divergence — return is a resume, not a copy

### 6.1 The return path (quota reset is the trigger)

When the original account's window resets, the app suggests — never
executes — the return (automation invariant: prepares/user executes,
`docs/SPEC.md:26,263`; ADR-0003 via Prior-09 D6). Execution:
(1) **resume the original native session** (`claude --resume <id>`,
`codex exec resume`, `zcode --resume`, `docs/SPEC.md:271`) — the
original transcript was never moved (copy-never-move,
`docs/SPEC.md:287`), so nothing is copied back; (2) inject *one*
delta turn: "While you were away" — turns, decisions, diff stat,
and todo changes from the away epoch(s), rendered from their packs;
(3) record a `switch-back` edge whose `resumed_from_epoch` points at
the original epoch (§2.1). Why this matters: a copy-back (the naive
reading of SPEC `docs/SPEC.md:288`) would fork the original
session's identity, break its `sessionId` chain, and double-count
usage on uuid re-ingest (Prior-09 §Q3 dedupe contract). Resume
keeps one native identity per epoch-line — that is what makes the
*N*-th switch-back identical in cost to the first. Claude v2.1.234+
can self-wait for reset (`docs/research/claude-code.md:53`); the
engine's value-add is therefore not babysitting one session but the
cross-account/cross-provider return with delta (Prior-09 §Q4).

### 6.2 Both sides progressed — three-way reconciliation

After switch-back is *offered*, the common case is divergence: the
user (or a still-running away instance) added turns on B while A
also advanced, or the worktree changed under both. Prior work stops
at naming this (Prior-09 F-H-11). Rule set:

- **Common ancestor** = the pack/manifest at the fork edge; its
  `message_uuids_sha256` + uuid set is the base (`state.json`, §4).
- **Classify:** away-only turns (uuids ∉ base, ∈ B) → delta turn,
  auto-includable; original-only turns → already native, nothing to
  do; *same-file edits on both sides* (diff-vs-diff on one path) →
  conflict list, never auto-merged; todo/state conflicts → listed.
- **User choices per conflict:** Keep original / Take away version /
  Keep both (away version saved as `…away-<date>` sibling + a turn
  pointing at it). Conversation conflicts are never text-merged:
  transcripts are trees (`uuid`/`parentUuid`,
  `docs/research/claude-code.md:41`), so "merge" = delta injection,
  not interleaving. The merge edge records both parents (§2.1) and
  the choice per conflict in `manifest.json`.
- If the away instance is **still running**, switch-back first
  offers freeze-or-fork (§5.2); resuming A while B runs is legal
  (parallel accounts are the founding use case,
  `docs/PROPOSAL.md:51`) but the UI must say both are now live on
  one work item — silence here is how wrong-account billing happens
  (Claude #87447, cited `docs/PROPOSAL.md:52`).

## 7. Destination ranking — cost/quota-aware, explainable, never silent

Adopt Prior-09 §Q6 wholesale as the base (hard filters, then
`0.45·headroom + 0.20·affinity + 0.15·fidelity + 0.10·costFit +
0.10·calm`, confidence-discounted, explained in words). Pass 1
under-weights three switching-specific costs; extend, don't replace:

- **Switch cost (new term, −):** replay tokens for this work item on
  this target (pack `replay_tokens_est`, §4) priced at the target's
  rates — a 200k-token replay onto a nearly-full 5h window can be
  worse than a lower-fidelity pack onto an empty one. Transplant and
  native resume cost ≈ 0 here; that asymmetry must be visible.
- **Return likelihood (new term, +):** if the source resets in
  < 60 min (reset timers, `docs/SPEC.md:257`), prefer targets that
  maximise *returnability* (same provider > replay-only), because a
  cheap hop now plus a lossless return beats a marginally better
  one-way hop. This is the bidirectional correction to §Q6, which
  scores only the outbound leg.
- **Cumulative hop cost:** the work item's spent replay tokens (§2.2)
  are shown on the ranking screen ("3 switches so far ≈ N tokens");
  hiding compounding cost across unlimited hops would make
  "unlimited" a billing trap (quota opacity is the #1 pain,
  `docs/PROPOSAL.md:23`).

Hard filters stay eliminative (logged-out, exhausted with no reset
before need-by, wrong mechanism for the request — Prior-09 §Q6).
Ranking *suggests*; execution is always the user's click
(`docs/SPEC.md:26`; `docs/PROPOSAL.md:63`). Auto-rotation on quota
is prohibited product-wide (`docs/PROPOSAL.md:63`;
`docs/research/landscape-and-stack.md:101-102`) — ranking copy must
never say "we switched you", only "you switched".

### 7.1 User warning copy (verbatim — implement these strings)

Terminology per §1. `{…}` = interpolated. Each string names what
does *not* carry; generic "some context may be lost" is banned.

| Situation | Copy |
|---|---|
| Transplant, proven pair | "Continues on {Account B} with the full conversation. Running tools and subagents stop; {Account A} stays resumable." |
| Transplant, unproven (v0 today) | "Experimental: only the main transcript moves. Subagent files and rewind history stay on {Account A}. {Account A} stays resumable." (Prior-09 D1) |
| Replay cross-provider | "Fresh start on {Provider B}: it re-reads the full conversation as text ({N} turns, ≈{T} tokens). Native todos and tool state don't carry — they're rebuilt from the story." |
| Compacted source | "This conversation was compacted. Early detail carries as text, not as live memory — {Provider B} may ask about early decisions." (Prior-09 §H2.1) |
| Target quota-tight | "{Account B} has ≈{X}% left (≈{TTE} at its recent pace). {Account C} has more room. Switching never happens automatically — your call." |
| Both sides progressed | "Both sides moved since the switch. {n} new turns and {m} file conflicts need your choice — nothing is merged silently." (§6.2) |
| Switch-back ready | "{Account A} has reset. Resume the original conversation there? Work from {Account B} arrives as one 'while you were away' update." (§6.1) |
| Same-branch parallel | "{Instance A} and {Instance B} are both live on branch {b}. Edits can collide — give {Instance B} its own worktree?" (§5.3) |
| Failure, source safe | "Switch didn't complete — {reason}. Nothing changed on {Account A}; you can resume it exactly where you left it." (§9) |

## 8. One click vs wizard — decision tree (resolves critic C4)

Critic C4: prior parts specify 3-step and 7-step wizards for the
same flow (`docs/review/deep/13-critic-gaps-and-crosschecks.md:148-156`).
Resolution: there is no fixed step count. The door is chosen by
*risk computed at preflight* (§5.1); the wizard renders only the
steps whose data is non-trivial.

```
Is target the usual account for this project AND mechanism =
  transplant/native-resume AND fidelity = F3/F4 AND no compaction,
  no conflicts, no listed Drops, target headroom ≥ Tight?
├─ YES → 1-CLICK: button "Switch to {Account} ▸" → confirm popover
│        (carry-list one-liner + cost) → execute. Undo toast 10s
│        ("Switched — Undo" = resume source, §9).
└─ NO  → WIZARD, steps included only if needed:
         1 Target (ranked rows, §7 — always)
         2 What's carried (carry-list + pack preview — always;
           Prior-09 §H5: no execution from an unseen preview)
         3 Conflicts (only if §6.2 finds any)
         4 Rules/diff (only if rules or worktree diverged, §5.3)
         5 Confirm (cost, fidelity label, target headroom — always)
```

1-click is therefore not a lesser wizard; it is the wizard with all
conditional steps proven empty. Telemetry distinguishes
`switch_oneclick` vs `switch_wizard{steps_shown}` so the team can
prove the fast path is earned, not assumed. Keyboard: `⌘⇧H` switch,
`⌘R` resume/switch-back (extends the shortcut debt noted FCUR F2.x;
exact bindings owned by the screen spec, not this file — dedupe
rule, critic `docs/review/deep/13-critic-gaps-and-crosschecks.md:259`).

### 8.1 Switch history / lineage graph UI

Surface: Project panel (the juggling anchor, ADR-0007
`docs/adr/0007-instance-centric-information-architecture.md:31-37`;
SPEC Projects view `docs/SPEC.md:325-330`). Rendering rules:
horizontal timeline per work item; epoch node = provider icon +
account label + instance kind badge + state chip (indicators the
maintainer demands, `docs/internal/expected-workflow.md:13-14`); edge =
labelled arrow (`transplant F3 · 12:04` / `replay F1 · ≈41k tok` /
`switch-back ↺`); fork = branching lines; merge = converging lines
with conflict-count badge; failed edge = dashed red, clickable to
the §9 reason. Clicking any node offers Resume (if its native
session exists), Switch from here…, and Export pack…. The graph —
not a session list — is where "how did this work travel?" is
answered; lists stay for Instances/Sessions (SPEC `docs/SPEC.md:331-348`).
Scale guard: collapse epochs older than the last fork/merge into
"+n switches" with expand; never render unbounded raw lists (the
unbounded-history flaw FCUR found in project session rows).

## 9. Failure and rollback — source is never the casualty

Extends Prior-09 §H4 (F-H-01…F-H-12); every row there stays in force.
Switching adds cycle-specific failures; each leaves the source
resumable and writes a failed edge (§2.1) — silent partial state is
the only unforgivable outcome.

| ID | Failure (stage) | Engine behaviour | Rollback |
|---|---|---|---|
| S-F1 | Wrong transcript resolved (Resolve) | Refuse if `transcript_ref` missing/ambiguous; never newest-mtime (v0 flaw, `commands.rs:1026-1033`) | Nothing moved; user picks the file |
| S-F2 | Torn tail / still writing (Freeze) | §5.2 two-choice freeze; parse-verify tail (Prior-09 F-H-02) | Resume source; no edge beyond `Failed{freeze}` |
| S-F3 | ID collision on target (Stage) | Fork: new native id + rewritten chain (Prior-09 F-H-03; SPEC `docs/SPEC.md:280`) | Fork is the rollback-safe path; original target untouched |
| S-F4 | Crash mid-copy (Stage) | Temp-dir stage → verify (sha256 vs manifest) → atomic rename (Prior-09 F-H-10) | Delete staged copy; source untouched by construction |
| S-F5 | Target launch fails (Launch) | Keep staged artifacts 24h for retry; mark edge `Failed{launch, reason}` | Resume source (T1); retry reuses the verified stage |
| S-F6 | Target quota dies during replay prime | Abort prime; charge is disclosed, not hidden (§7 switch cost) | Resume source; edge records tokens burned |
| S-F7 | Switch-back target session deleted/expired | Offer nearest surviving epoch or pack-primed fresh instance, labelled F0/F1 | Never fabricate a resume; graph shows the dead node greyed |
| S-F8 | Merge conflict unresolved (Return) | Work item stays forked; both epochs resumable | No forced merge; fork persists until the user chooses (§6.2) |
| S-F9 | N-hop loop (A→B→A→B…) with growing replay cost | Allowed — user-initiated — but cumulative-cost line (§7) turns amber; suggest transplant-capable target | Cost honesty, not a block (advisor informs, never blocks, `docs/SPEC.md:237`) |

Undo semantics (all doors): "Undo switch" within the toast window
= stop target epoch + resume source epoch + compensating edge
(Prior-09 §H7). After the window, the same actions remain available
from the graph (§8.1) — undo is a navigation, not a time bomb.

## 10. Acceptance criteria and test matrix per pair

Ship gate (extends Prior-09 §H9; blocks marketing, not dogfooding):
a pair may advertise its §3.4 label only when its row below is
green in CI fixtures *and* one live dogfood cell passes. Fixtures
H-F1…H-F8 (Prior-09 §H8) are prerequisites for every row.

| Pair (ordered) | Fixture tests (CI, fake harness + goldens) | Live dogfood cell | Advertised ceiling when green |
|---|---|---|---|
| Claude→Claude (diff account) | Sibling set, torn tail, collision fork, compacted→F2, pathological cwd (Prior-09 H-F1-F6); A→B→A→B 4-hop uuid-dedupe | 2 Max accounts, subagent-heavy repo; return on reset resumes original id | F3 (F2 if compacted) |
| Codex→Codex | Rollout copy + `exec resume` pickup; absolute-`cwd` path check (`docs/research/codex.md:46`) | 2 Codex accounts, same checkout | F3 (pending binary proof; today F1) |
| ZCode→ZCode | SQLite clone against 2 schema versions, or stays blocked (`docs/SPEC.md:282`) | 2 GLM plans, CLI-only (`docs/research/zcode.md:85`) | F1 until clone proven |
| Claude→Codex / Codex→Claude | IR golden round-trip; tool-outcome distill assertions; rules-ring conversion goldens (EPICS M4-4/M4-6, `docs/plan/EPICS.md:106-108`) | Continue a Claude task in Codex; next answer uses pre-switch context only | F1 |
| Claude→ZCode / ZCode→Claude | Replay-from-SQLite goldens; prime-path probe for ZCode target | Source-side F1 demo; target pack-open | F1 source / F0 target |
| Codex↔ZCode | IR goldens both ways | One cross continuation each way | F1/F0 as §3.3 |
| Any→Gemini/OpenCode (roadmap) | Pack-open smoke only; no replay claim | Pack imported manually | F0 |
| Switch-back (all proven pairs) | A→B→A: original `native_id` resumed, delta turn present, zero duplicate uuids; both-progressed merge fixture (§6.2) | Quota-reset return judged continuous by the user (PROPOSAL round-trip ≥80%, `docs/PROPOSAL.md:144`) | Same as outbound pair |
| Ranking/doors | §Q6 explanation snapshot tests + §8 door selection per risk fixture | — | n/a (gate for UI ship) |

Global acceptance: (1) source bytes hash-identical after every
test; (2) no edge executes without a user click — guardrail suite
(`docs/SPEC.md:26,441`; `docs/plan/EPICS.md:109` M4 gate); (3) the
UI never says "lossless/exactly" below F4 (Prior-09 §H9); (4) an
A→B→A→C→B graph survives restart (§2.3). Failure of any cell
*lowers that pair's advertised ceiling one level until fixed* —
the matrix is the capability flag's source of truth (Prior-09 §H8),
which also finally justifies the `TRANSPLANT` bit
(`crates/juggler-adapters/src/claude_code.rs:111-120`): it may stay
only for pairs whose row is green (Prior-09 D4).

## 11. Build order, risks, unresolved

Order (respects EPICS M4 scope, `docs/plan/EPICS.md:99-109` — this
file adds no milestone, it deepens M4): (1) honest relabel + sibling
set for Claude→Claude (fixes the shipped over-claim first, FCUR
F3.2); (2) packs + manifest for every edge (§4); (3) Switch Graph
storage + Project-panel timeline (§2.3, §8.1); (4) switch-back as
resume+delta (§6.1); (5) divergence handling (§6.2); (6) ranking
terms + doors (§7-§8); (7) cross-provider replay pairs in §10
order. Risks: native formats churn weekly (mitigation: per-version
parsers + fixture canary, `docs/SPEC.md:184,437`; `docs/PROPOSAL.md:132`);
ZCode surfaces undocumented (degrade to F0, never reverse-engineer,
`docs/SPEC.md:133`); replay cost surprises (mitigation: §7 cost
terms + S-F9). Unresolved, honestly: ZCode prime path and SQLite
clone safety; Codex cross-account resume against the real binary;
Gemini/OpenCode adapter shapes; whether Claude sibling-file copy
survives the next format change — each is a §10 cell to prove, not
a paragraph to write. What is *not* unresolved: the shape of the
answer — graph, pack, resume-don't-copy, ranked-never-silent — which
is what "as many times I want, back and forth"
(`docs/internal/expected-workflow.md:7`) demands of the design.





# 03 — Smart Onboarding & Provider Setup (Second Pass)

> Second pass over the canonical maintainer workflow in
> [expected-workflow.md](../internal/expected-workflow.md#L11)
> ("fully smart and automated onboarding wizard … scanning directories,
> existing providers … making recommendations"). This file does **not**
> restate the prior report; it cites it, then extends or corrects it.
> Prior sources: Maximum-Depth Report (cited as `MDR §…`),
> [full-competitive-ux-review.md](full-competitive-ux-review.md)
> (cited as `FCUR F…`), and
> [03-ui-onboarding-handover-settings-audit.md](deep/03-ui-onboarding-handover-settings-audit.md)
> (cited as `Deep-03 §…`). Every claim carries a repo `file:line`,
> a prior-report section, or an external URL.

## 0. Second-pass charter — what was under-weighted, wrong, or missed

The prior research audited the wizard that exists. The maintainer's
ask in [expected-workflow.md](../internal/expected-workflow.md#L11)
is broader: onboarding for *first install, new project, add provider,
add account, add instance* as one near-zero-config intelligence.
Six gaps follow; the rest of this file designs the fix.

| # | Verdict | Prior position | Second-pass correction |
|---|---|---|---|
| P2-1 | Under-weighted | "Protect onboarding" (`FCUR F2.6`; `MDR` Executive #8) | Protect the *interaction*, not the *scope*: scope is Claude-only and must grow (§1) |
| P2-2 | Wrong by omission | Scan = 4 Claude origins (`Deep-03 §1.2`) | Scan misses shell aliases, installed CLIs, cc-switch DB, Codex/ZCode homes (§3) |
| P2-3 | Missed | No inference/confidence model anywhere | Found configs need labelled inference, not raw cards (§4) |
| P2-4 | Missed | Flows (b)–(f) undesigned | New-project / add-Provider / add-Account / add-Instance / import are distinct flows (§6) |
| P2-5 | Wrong framing | "Account" wizard, singular (`OnboardingWizard.tsx:79`) | Maintainer language is Providers / Accounts / Instances; domain says Harness/Profile/Session (§2) |
| P2-6 | Under-weighted | Login leak noted (`Deep-03 §3`) | Login *state taxonomy* (partial/expired/SSO/API-key) is the real design surface (§7) |

**P2-1 evidence.** The shipped wizard hard-codes one harness:
`const HARNESS = "claude-code"` at
[OnboardingWizard.tsx](../../ui/src/screens/OnboardingWizard.tsx#L18),
so the "extend, never fork" advice in `FCUR F2.6` is right about the
shell and wrong if read as "scope is done". SPEC already promises a
harness-generic scan at
[SPEC.md](../SPEC.md#L414).

**P2-2 evidence.** `scan_claude_environment` assembles only vendor
default, `$CLAUDE_CONFIG_DIR`, manager roots, and `.claude*` home
matches at
[claude_environment.rs](../../crates/juggler-adapters/src/claude_environment.rs#L56).
Nothing reads shell rc aliases — yet aliases are the dominant
community multi-account pattern per
[claude-code.md](../research/claude-code.md#L33)
(canonical gist: https://gist.github.com/KMJ-007/0979814968722051620461ab2aa01bf2).

**P2-5 evidence.** The maintainer names the trio explicitly at
[expected-workflow.md](../internal/expected-workflow.md#L11)
("we should call them providers, accounts, and instances of the
tool"). No `provider` symbol exists in `crates/` or `ui/src/`
(grep-verified this pass); §2 maps the vocabularies instead of
renaming storage blindly.

## 1. Terminology — adopt maintainer language, map, flag conflicts

### 1.1 Canonical mapping (UI language ←→ domain/storage)

| Maintainer term (UI) | Domain term today | Code anchor | Meaning |
|---|---|---|---|
| Provider | Harness / adapter | [model.rs](../../crates/juggler-core/src/model.rs#L49) `HarnessId` | The tool vendor+CLI: Claude Code, Codex, ZCode |
| Account | Account | [model.rs](../../crates/juggler-core/src/model.rs#L145) `Account` | One login you own under a Provider, isolated state dir |
| Instance (tool instance) | SessionRecord + InstanceKind | [model.rs](../../crates/juggler-core/src/model.rs#L389), `:323` | One running/tracked occurrence (Terminal/Desktop) |
| *(no maintainer word)* | Profile | [model.rs](../../crates/juggler-core/src/model.rs#L173) `Profile` | Account + settings overlay (model/MCP/env) |
| *(no maintainer word)* | Session (conversation) | ADR-0007 Decision 1 | The resumable work conversation inside an Instance |

ADR-0007 already moves product language toward Instances at
[0007-instance-centric-information-architecture.md](../adr/0007-instance-centric-information-architecture.md#L38);
this pass completes the trio by promoting **Provider** to the UI
name for what code calls *harness*.

### 1.2 Naming conflicts — flagged, with rulings

1. **Provider ≠ Codex `model_provider`.** Codex config uses
   `model_providers.<id>` for API endpoints per
   [codex.md](../research/codex.md#L18).
   Ruling: UI "Provider" always means the harness vendor; Codex
   endpoint config stays "model provider (advanced)" inside Profile
   settings, never shortened to Provider.
2. **Instance ≠ Fleet instance.** SPEC §11 uses "instance" for an
   AI Juggler node at
   [SPEC.md](../SPEC.md#L368).
   Ruling: UI says **Tool Instance** vs **Machine**; code keeps
   `InstanceKind` for tools and `MachineId` at
   [model.rs](../../crates/juggler-core/src/model.rs#L66)
   for fleet nodes. Never bare "instance" in fleet copy.
3. **Profile is invisible to the maintainer's trio.** The wizard
   silently creates a default `Main` profile (SPEC §12.1 step 3,
   [SPEC.md](../SPEC.md#L423)).
   Ruling: keep Profile as an *advanced* concept — every Account
   gets `Main`; Profiles surface only under "Customize" (§6).
4. **Session vs Instance in storage.** `SessionRecord` *is* the
   instance record today (comment at
   [model.rs](../../crates/juggler-core/src/model.rs#L386)).
   Ruling: rename in UI copy only this milestone; storage rename is
   a separate migration, explicitly out of onboarding scope.

## 2. Design principle — near-zero-config intelligence

The maintainer's bar is "it can't have thousands of options, it
needs to do most of the figuring out on its own" at
[expected-workflow.md](../internal/expected-workflow.md#L11).
Operationalised as four rules the prior report never stated:

- **R1 Scan before ask.** No question the filesystem, env, or a
  metadata probe can answer is ever typed by the user. Today's
  wizard asks label+plan even on import
  ([OnboardingWizard.tsx](../../ui/src/screens/OnboardingWizard.tsx#L233));
  both become *confirmations* of inferred values (§4).
- **R2 One recommendation, visibly reasoned.** The wizard proposes
  a complete setup ("3 Claude accounts, 1 Codex — here's your
  juggling set") with per-item confidence; the user accepts,
  edits, or expands. This extends Deep-03's per-card fix
  (`Deep-03 §1.3`) into a set-level proposal.
- **R3 Progressive disclosure.** Default path ≤3 decisions;
  everything else (Profiles, MCP sync, env overlays) lives behind
  "Customize" / "Expert setup". Escape hatches in §6.7.
- **R4 Never credentials.** Scanning stays metadata-only per
  ADR-0006 Decision 1 at
  [0006-smart-onboarding-scan-and-import.md](../adr/0006-smart-onboarding-scan-and-import.md#L28)
  and the probe contract at
  [environment.rs](../../crates/juggler-adapter/src/environment.rs#L9).
  The one existing exception — byte-copying `.credentials.json`
  on import at
  [claude_environment.rs](../../crates/juggler-adapters/src/claude_environment.rs#L224)
  vs SPEC §8 at
  [SPEC.md](../SPEC.md#L310) —
  must be resolved per `FCUR F4.1` option (b) *before* the scan
  widens; a wider scan must not widen the credential surface.

## 3. Scan engine — what to probe (never credentials)

Today: one adapter, four origins, Claude-only
([claude_environment.rs](../../crates/juggler-adapters/src/claude_environment.rs#L50),
trait default at
[adapter.rs](../../crates/juggler-adapter/src/adapter.rs#L80)).
Target: a per-Provider probe table run in parallel, each row
producing `DiscoveredInstall`-shaped evidence
([environment.rs](../../crates/juggler-adapter/src/environment.rs#L61))
plus new signal kinds. All probes are existence/size/mtime or
`--version`/`--help` execution — never file *contents* of any
credential file, never keychain *reads* (existence only, as in
[claude_code.rs](../../crates/juggler-adapters/src/claude_code.rs#L332)).

| # | Signal | Probe (metadata only) | Yields | New? |
|---|---|---|---|---|
| S1 | Config dirs | Existing 4 origins per Provider (`~/.claude`, `~/.codex`, `~/.zcode` + env overrides + home patterns) | Candidate Accounts | Exists (Claude) |
| S2 | Env overrides | `$CLAUDE_CONFIG_DIR`, `$CODEX_HOME` in app env + shell rc export lines (name match only) | Manual/manager setups | Partial (`:58`) |
| S3 | Other managers: t3 | Children of `ClaudeCodeProfiles` ([claude_code.rs](../../crates/juggler-adapters/src/claude_code.rs#L223)) | ManagerTool installs | Exists |
| S4 | Other managers: cc-switch | `~/.cc-switch/cc-switch.db` existence + provider-row *count* via SQLite metadata (no token columns) | Provider/Account inventory | **New** |
| S5 | Shell aliases/functions | Parse `~/.zshrc`, `~/.bashrc`, `~/.config/fish/*` for `CLAUDE_CONFIG_DIR=`/`CODEX_HOME=` assignments (variable *values* = paths only) | Forgotten manual accounts | **New** |
| S6 | Installed CLIs | `discover()` per adapter: PATH + known bins + `--version` ([claude_code.rs](../../crates/juggler-adapters/src/claude_code.rs#L126)) | Provider availability + Step-0 gate | Exists, unwired in wizard (`Deep-03 §3.2`) |
| S7 | Login metadata | Per-dir: `.credentials.json` non-empty?, keychain-entry existence, `auth.json` presence for Codex ([codex.md](../research/codex.md#L17)) | Login state per candidate | Exists (Claude) |
| S8 | Usage recency | Newest transcript mtime + session counts per dir (extends `newest_mtime_secs` at [claude_environment.rs](../../crates/juggler-adapters/src/claude_environment.rs#L329)) | "Your active account" ranking | Partial |
| S9 | Projects | Recent cwds from transcript paths / `projects/` children names (dir names only) | New-project flow seeds (§6b) | **New** |
| S10 | Desktop harnesses | `/Applications/*.app` presence for desktop Providers (ADR-0007 Decision 5) | Desktop Instance candidates | **New** |

cc-switch as a scan source is grounded in `MDR` deep-06 §5
(cc-switch stores providers in `~/.cc-switch/cc-switch.db`:
https://github.com/farion1231/cc-switch) and the landscape table at
[landscape-and-stack.md](../research/landscape-and-stack.md#L12).
Reading its DB is a *new read surface* requiring the same
metadata-only discipline: table/row counts and provider names,
never API keys it stores. If that line cannot be held in code
review, ship S4 as existence+count only.

**Scan contract additions** (trait-level, default-implemented so
adapters adopt incrementally, mirroring ADR-0006 Consequences at
[0006-smart-onboarding-scan-and-import.md](../adr/0006-smart-onboarding-scan-and-import.md#L71)):

- `scan_shell_aliases() -> Vec<AliasHint>` — path-valued env
  assignments with source file + line; parse is regex-scoped to
  the two known variable names, not general shell evaluation.
- `scan_managers() -> Vec<ManagerHint>` — t3/cc-switch presence.
- Every probe gets a 2 s budget and a `ScanHealth` result
  (`Ok | Partial(reason) | Denied`) — fixing Deep-03 §1.2 finding 2
  ("scan failure is impossible by construction, therefore
  invisible") at the contract level, not just in copy.
- Scan runs off the UI thread with progress (`n of 10 checks`);
  today's unbounded spinner is `Deep-03 §1.1` row 1.

## 4. Inference rules + confidence display

The prior report shows found dirs as neutral cards
(`Deep-03 §1.1`); it never asks *what the app should conclude*.
Each candidate gets an inferred identity + a confidence tier,
rendered inline — never a bare path. Inference is rules-only
(no model call): onboarding must work offline and deterministically.

### 4.1 Rule table (evaluated in order; first match wins per field)

| Field | Rule | Example inference | Tier |
|---|---|---|---|
| Account purpose | Dir/alias name contains `work|corp|acme` → Work; `personal|home|side` → Personal; else basename humanised | `~/.claude-work` → "Work" | High / Med |
| Account purpose | cc-switch provider name or t3 profile folder name | Folder `acct-2` → keep verbatim, tier Low | Low |
| Plan tier | Settings/quota feed presence (Claude statusline history) or user-confirmed last time; **never** guessed from usage volume | No evidence → "Not sure yet" (today's default at [OnboardingWizard.tsx](../../ui/src/screens/OnboardingWizard.tsx#L21)) | — |
| Login state | S7 probe: credential file non-empty OR keychain entry exists → Signed in; file exists but empty/stale mtime → "May be expired" | Extends binary probe at [claude_code.rs](../../crates/juggler-adapters/src/claude_code.rs#L332) | High / Med |
| Active vs dormant | Transcript mtime ≤7 d → Active; ≤90 d → Dormant; else Archive candidate | Uses S8 | High |
| Duplicate | Same canonical dir via 2+ signals (alias + home-pattern) → one candidate, signals merged | Dedupe exists for paths at [claude_environment.rs](../../crates/juggler-adapters/src/claude_environment.rs#L83); extend to signal-merge | High |
| Already imported | Managed marker present ([claude_code.rs](../../crates/juggler-adapters/src/claude_code.rs#L377)) → "Already in AI Juggler" | Fixes `Deep-03 §1.2` finding 1 in the model, not just copy | High |
| Project link | Transcript cwd set (S9) → "Used on: api, web-app" | Dir-name only | Med |

Confidence tiers render as words, not scores: **Found** (direct
probe), **Likely** (name/pattern inference), **Guessed** (weak
signal, always editable). Every inferred field is one-tap editable
inline; editing never re-runs the scan. Plan tier is deliberately
*not* inferred from token volume — quota honesty tiers
(Measured/Computed/Estimated at
[model.rs](../../crates/juggler-core/src/model.rs#L288))
forbid manufacturing precision the probe cannot support. Note the
shipped plan list also omits Max 5× (`Deep-03 §2.1`,
[OnboardingWizard.tsx](../../ui/src/screens/OnboardingWizard.tsx#L20));
add it when plan becomes confirm-not-type (§6).

### 4.2 Confidence display — wireframe (Detect, proposed)

```
┌─ Welcome to AI Juggler ──────────────────────────────┐
│ We scanned your machine (nothing was changed).        │
│ Found 4 setups · 2 need a quick sign-in · Details ▾   │
│ ┌───────────────────────────────────────────────────┐ │
│ │ Claude · ~/.claude            ● Signed in  Found  │ │
│ │ Likely: Work · Active this week · 3 projects      │ │
│ │ [ Import as “Work” ]  [ Edit ]                    │ │
│ └───────────────────────────────────────────────────┘ │
│ ┌───────────────────────────────────────────────────┐ │
│ │ Claude · ~/.claude-side   ○ Not signed in  Found  │ │
│ │ Guessed: Personal · Dormant 40 d                  │ │
│ │ [ Import ]  [ Edit ]  [ Ignore ]                  │ │
│ └───────────────────────────────────────────────────┘ │
│ ┌───────────────────────────────────────────────────┐ │
│ │ Codex · ~/.codex              ● Signed in  Found  │ │
│ │ 1 account · CLI v0.44 installed                   │ │
│ │ [ Import ]  [ Edit ]                              │ │
│ └───────────────────────────────────────────────────┘ │
│ [ Use recommended setup → ]   Add manually · Expert…  │
└───────────────────────────────────────────────────────┘
  signals merged, inference labelled, managed/disabled
  states inline — supersedes Deep-03 §1.3 (per-card only)
```

## 5. Recommendation engine — from inventory to a juggling set

Input: the §4 inventory. Output: one proposed set the user can
accept in a single click. This is the piece the maintainer's
"making recommendations" clause at
[expected-workflow.md](../internal/expected-workflow.md#L11)
demands and no prior section designs (closest: `MDR` Executive #5
needs-attention inbox — a *daily* surface, not an onboarding one).

**Rules (deterministic, explainable in one line each):**

1. **Import every Found+Signed-in candidate** as its own Account;
   default ON. Rationale shown: "Signed in — import keeps history".
2. **Import Dormant candidates default OFF**, listed under
   "Found but quiet" with one-tap include. Prevents clutter the
   maintainer warned against ("thousands of options", *ibid.*).
3. **One fresh Account per installed-but-unconfigured Provider**
   (S6 positive, S1 empty): proposed as "Add Codex (installed,
   not set up)" — never auto-created without the click.
4. **Label by inferred purpose; suffix collisions** (`Work`,
   `Work 2`) — fixes the duplicate-label hole at `Deep-03 §2.1`
   in the engine, not with a validation error after typing.
5. **First Project suggestion:** most-recent cwd (S9) becomes
   "Start here" on the Summary; the juggling premise needs a
   project, and `Deep-03 §4` notes the funnel ends without one.
6. **No recommendation ever crosses the automation line:**
   recommendations *stage*; creation/login execute only on the
   accept click (SPEC automation invariant at
   [SPEC.md](../SPEC.md#L26)).

**Worked example** (the brief's case): scan finds 3 Claude
config dirs + 1 Codex home → proposal reads "Your juggling set:
Claude ×3 (Work — signed in; Personal — signed in; Side —
needs sign-in) + Codex ×1 (signed in). Import 3, sign in 1
later?" with `[ Accept setup ] [ Customise ]`. Accept executes
imports sequentially with per-item progress and the §8
verification checklist; failures degrade per-item (§7), never
abort the set. Post-accept, the Summary offers the two exits
Deep-03 §4 found missing: **Launch first Instance** and
**Add another Account** loop-back.

## 6. Flows (a)–(f) — button-by-button wizard specs

Shared shell for all six flows: modal wizard (keep today's
`fixed inset-0` shell at
[OnboardingWizard.tsx](../../ui/src/screens/OnboardingWizard.tsx#L109),
per `FCUR F2.6` "protect the interaction"), step dots, Back where
reversible, coded errors (never raw `to_string()` — the blocker
at `Deep-03 §0`), auto re-probe on terminal exit (picker P0-1,
`Deep-03 §9`). Click budgets count primary-path taps from entry
to Done, excluding vendor's own OAuth screens. "Expert" escape:
every flow exposes **Expert setup…** (raw state-dir path, env
overlay, Profile settings) — visible, never in the default path.

### (a) First install — target ≤5 clicks, 0 typed fields required

| Step | Controls (button-by-button) | Clicks |
|---|---|---|
| 0 Install gate | Per-Provider `discover()` rows; [Install Claude Code →] opens vendor install page; [Continue without it] per Provider | 0–1 |
| 1 Scan + proposal | §4.2 screen: [Use recommended setup →] primary; per-card [Import]/[Edit]/[Ignore]; [Add manually]; [Rescan] | 1 |
| 2 Confirm identity | Inferred labels/plans pre-filled; [Looks good →]; inline edit per row | 1 |
| 3 Sign-ins | Queued per Account needing login: [Sign in] (embedded terminal), [Open vendor page], [Copy command], [Skip for now]; auto-advance on probe success | 1 ea. |
| 4 Summary | [Launch first session →] primary; [Add another account]; [Done] | 1 |

Step 0 is new: today a missing `claude` binary fails only at
Login (`Deep-03 §3.2`, `Deep-03` E1). Gate copy uses the searched
paths from `discover()` at
[claude_code.rs](../../crates/juggler-adapters/src/claude_code.rs#L134).
Zero-found case: proposal collapses to "Create your first
account" with Provider picker (installed first, others greyed
with install links) — never the dead-end dashed box at
[OnboardingWizard.tsx](../../ui/src/screens/OnboardingWizard.tsx#L159).

```
┌─ Your juggling set ──────────────── ●●○○ ─┐
│ Claude ×3 + Codex ×1 — ready to import     │
│  ✓ Work (Claude)      signed in            │
│  ✓ Personal (Claude)  signed in            │
│  ○ Side (Claude)      sign in later        │
│  ✓ Main (Codex)       signed in            │
│ Start project: ~/Dev/api (most recent) ▾   │
│ [ Accept setup → ]  [ Customise ]  Expert… │
└────────────────────────────────────────────┘
```

### (b) New project — target ≤3 clicks (the daily juggling entry)

Entry: Projects rail "New project" + Summary "Start here" (§5.5).
This flow is *absent* from prior onboarding scope — the wizard
ends at Account-ready (`Deep-03 §4`) while ADR-0007 makes Project
the juggling anchor at
[0007-instance-centric-information-architecture.md](../adr/0007-instance-centric-information-architecture.md#L31).

| Step | Controls | Clicks |
|---|---|---|
| 1 Pick folder | Recent dirs (S9) as tiles; [Choose folder…] native picker; drag-drop | 1 |
| 2 Pick starting set | Recommended Account (last-used on this dir, else most headroom) pre-selected; [Change] opens Account list; [Start session →] | 1–2 |
| Done | Instance launches; project pinned with Account history row | — |

No Account yet → inline detour: "No account for Claude yet —
[Add account] (2 min)" launching flow (d) modally, then return.
Never typed paths (parity with NewSession recents at `FCUR F2.7`).

### (c) Add Provider — target ≤4 clicks

Entry: Accounts header [Add Provider]. Distinct from (d): a
Provider is a *tool*, not a login (maintainer trio, §1).

| Step | Controls | Clicks |
|---|---|---|
| 1 Pick Provider | Cards per adapter: Installed vX.Y (Found) / Not installed [Install →]; verified-range badge from `verified_against` ([adapter.rs](../../crates/juggler-adapter/src/adapter.rs#L35)) | 1 |
| 2 First Account | Scan scoped to this Provider pre-run; [Import found] or [Add fresh]; label pre-inferred | 1–2 |
| 3 Sign in + Summary | Same login queue as (a) step 3; Summary with [Launch] / [Add another account on this Provider] | 1 |

Desktop Providers (S10) show kind badge Terminal/Desktop per
ADR-0007; a desktop Provider with single-instance limits is
labelled honestly before selection, not after a failed launch.

### (d) Add Account to an existing Provider — target ≤3 clicks

Entry: Provider card [Add account] + Summary loop-back. This is
today's wizard re-scoped: Provider locked (no harness picker —
fixing the hard-coded `HARNESS` at
[OnboardingWizard.tsx](../../ui/src/screens/OnboardingWizard.tsx#L18)
by *parameterising* it, not by adding a parallel wizard).

| Step | Controls | Clicks |
|---|---|---|
| 1 Source | Provider-scoped scan results; [Import this] per card; [Add fresh account] | 1 |
| 2 Identity | Label pre-inferred + collision-suffixed (§5.4); plan confirm (incl. Max 5×); [Create →] | 1 |
| 3 Sign in | Login queue (skip allowed); auto-advance on probe | 1 |
| Summary | [Launch] / [Add another] / [Done] | (1) |

Duplicate-import guard: a source already imported shows
"Already imported as «Work»" with [Open it] instead of [Import]
— requires import provenance on `Account` (field gap noted at
`Deep-03 §2.2`; `Account` today stores no source path at
[model.rs](../../crates/juggler-core/src/model.rs#L145)).

### (e) Add Instance — target ≤2 clicks (launch, not setup)

Entry: Project panel "Launch with…" + Instances [New instance].
An Instance is a *run*, so this flow must feel like launching,
not onboarding — the prior report's silence here is why "add
instance" kept collapsing into "new session" copy.

| Step | Controls | Clicks |
|---|---|---|
| 1 Launch sheet | Project (pre-filled from context), Account rows with quota headroom + login state, Profile = Main (Customize ▸ reveals Profiles/model/MCP); [Launch →] | 1–2 |
| Gate | Admission advisor verdict OK/Tight/No inline (SPEC §5.2 at [SPEC.md](../SPEC.md#L237)); Tight/No never blocks, always explains | — |

Logged-out Account rows are disabled with "Sign in first"
(one-tap detour into the login queue, then return). Quota
headroom uses honesty labels; unknown headroom renders
"headroom unknown", never a fabricated ring (`MDR` Executive #8;
today's `percent={0}` fabrication is `Deep-03 §7`).

### (f) Import from competitor / manual `CLAUDE_CONFIG_DIR` — ≤3 clicks

Entry: Detect card action + Expert "Import from folder…". Covers
three sources the current scan half-sees: cc-switch-managed
configs, t3 profiles (S3, exists), and hand-rolled
`CLAUDE_CONFIG_DIR` dirs found via aliases (S5, new).

| Step | Controls | Clicks |
|---|---|---|
| 1 Confirm source | Source path + origin badge (cc-switch / t3 / alias / manual); size + per-folder breakdown; [Import copy] / [Just track it*] | 1 |
| 2 Identity + sign-in | As (d) steps 2–3; keychain re-login note inline *here*, not discovered at Login (fixes `Deep-03 §2.3` caveat 2) | 1–2 |

\* "Just track it" (adopt-in-place, no copy) is **Expert-only and
off by default**: it breaks the managed-dir invariant (writes
stay outside `accounts/` at
[SPEC.md](../SPEC.md#L400))
and the read-only-source proof (`Deep-03 §2.3`). If offered at
all, it needs its own ADR; default answer is copy-import via
`import_account` at
[adapter.rs](../../crates/juggler-adapter/src/adapter.rs#L94).
Manual-folder import reuses the `is_claude_config_dir` evidence
check at
[claude_environment.rs](../../crates/juggler-adapters/src/claude_environment.rs#L294)
(and Codex/ZCode equivalents) — a folder that fails it gets a
plain-language reason + [Pick a different folder], never a raw
OS string (`Deep-03` E6/E8).

### 6.7 Expert escape hatches (every flow)

[Expert setup…] reveals, without leaving the wizard: raw state-dir
path (read-only display + [Reveal in Finder]), env overlay editor
(`SettingsOverlay.env` at
[model.rs](../../crates/juggler-core/src/model.rs#L161)),
Profile CRUD, "Import from folder…" (flow f), and a copyable
shell snippet reproducing the setup by hand (`CLAUDE_CONFIG_DIR=…
claude` — the community pattern at
[claude-code.md](../research/claude-code.md#L33)).
Escape hatches are logged as onboarding events (§9) so the team
learns which inferences experts keep overriding.

## 7. Error & edge catalogue (onboarding-specific)

Extends `Deep-03 §5` (E1–E14, incorporated by reference — do not
re-implement differently); rows below are new or re-scoped by
the wider scan. Every row: detection → user copy → recovery
button. No row may surface a raw backend string (blocker,
`Deep-03 §0`).

| # | Case | Detection | Recovery in-flow |
|---|---|---|---|
| N1 | Partial login (OAuth started, abandoned) | PTY exited, probe still LoggedOut | "Sign-in didn't finish" + [Reopen sign-in] [Skip] (extends E11) |
| N2 | Expired login | Credential present, stale/empty (S7 Med tier) | "May be expired — sign in again" queued, Account still created |
| N3 | Duplicate account (same source twice) | Provenance match (§6d) | Block second import; [Open existing] |
| N4 | Corporate SSO | Vendor flow redirects to IdP; probe lags | Copy: "Finish in your browser/IdP window, then return"; poll probe 2 s while step visible |
| N5 | API-key vs subscription | `auth.json`/env-key presence without OAuth markers (Codex, [codex.md](../research/codex.md#L18)) | Badge "API key account" — quota shows Estimated/none, never subscription rings |
| N6 | Two logins, one identity | Two dirs probe Signed-in with same plan+recency pattern | Warn (non-blocking): "These may be the same login twice — juggling needs two *subscriptions*" (`FCUR` Persona B wrong-account risk) |
| N7 | cc-switch DB locked/corrupt | SQLite open fails (S4) | Degrade S4 to existence-only; note in Scan Details, never fail scan |
| N8 | Alias points at deleted dir | Canonicalize fails (as [claude_environment.rs](../../crates/juggler-adapters/src/claude_environment.rs#L86)) | Show greyed "alias → missing folder" with [Ignore]; do not hide silently |
| N9 | Provider installed, unsupported version | `discover()` version ∉ `verified_against` | "Installed (vX, unverified)" — allow Account, badge Instances until verified (SPEC §4 drift rule at [SPEC.md](../SPEC.md#L184)) |
| N10 | Import > ceiling with breakdown | Per-folder sizes pre-copy (fixes E7) | [Import settings only — skip history] degraded import; [Add fresh] |
| N11 | Offline mid-onboarding | Login web/PTY fails + scan already done | Accounts persist; Summary marks "sign in when online"; no re-scan on reconnect |
| N12 | Scan permission-denied (home subdir) | `ScanHealth::Denied` per probe (§3) | "We couldn't check 2 places" + Details; empty-result copy forbidden (fixes `Deep-03 §1.2` #2) |

N6 deserves emphasis: the maintainer's core case is *multiple
subscriptions from the same lab*
([expected-workflow.md](../internal/expected-workflow.md#L5));
two dirs sharing one login silently defeats quota juggling, and
no prior section names it. Detection stays heuristic + honest
("may be"), never a block.

## 8. Verification checklist — proving isolation (per Account)

Run automatically after every import/create; results render on
the Summary as pass/fail lines, and any fail blocks "Launch"
(not "Done"). This operationalises the M1 acceptance "env +
keychain isolation verified" in
[EPICS.md](../plan/EPICS.md#L63)
and the guardrail suite `FCUR F4.2` demands.

- [ ] Managed dir exists, perms 0700, marker present
  (`prepare_managed_dir` contract, ADR-0006 Decision 2).
- [ ] Launch env points at the managed dir only
  (`CLAUDE_CONFIG_DIR`/`CODEX_HOME`/`HOME` per SPEC table at
  [SPEC.md](../SPEC.md#L130)).
- [ ] Source dir tree hash/mtime snapshot unchanged (import
  only; proof pattern in `Deep-03 §2.3`).
- [ ] Login probe scoped to *this* dir answers independently —
  signing out Account B must not flip Account A's chip.
- [ ] Two Instances (one per Account, same Provider) run
  concurrently; each transcript lands only in its own dir.
- [ ] Zero writes outside managed dirs during the whole flow
  (fixture-HOME guardrail test, `FCUR F4.2`).
- [ ] Credential contents never logged/telemetry-tagged;
  `.credentials.json` handling matches the resolved F4.1 rule.

## 9. Onboarding metrics (instrument from day one)

Events (local-first; telemetry opt-in per SPEC §8): `scan_started/
completed{providers,found,health}`, `proposal_shown/accepted/
customised`, `import_started/completed/failed{reason_code}`,
`login_queued/completed/skipped/expired`, `expert_opened`,
`first_launch{seconds_since_install}`. Targets, tied to the
success metric "install → two accounts in parallel < 5 min" at
[PROPOSAL.md](../PROPOSAL.md#L142):
time-to-first-Instance (p50 < 3 min on found-set machines),
proposal-accept rate, per-step drop-off (esp. Login queue),
inference-override rate per field (drives §4 rule tuning),
scan-health Denied rate. Report funnel weekly during beta;
a step losing >15% gets a Deep-03-style audit before new features.

## 10. Phasing (smallest shippable slices)

1. **Parameterise + gate:** wizard takes a Provider param (kills
   the constant at
   [OnboardingWizard.tsx](../../ui/src/screens/OnboardingWizard.tsx#L18));
   add Step-0 install gate + coded errors. Ships with Claude only.
2. **Infer + propose:** §4 rules + §5 set proposal for Claude;
   Summary gains [Launch] / [Add another]. No new signals yet.
3. **Widen scan:** S5 aliases + S9 projects, then S4 cc-switch
   (existence-only first); Codex/ZCode `scan_environment` impls
   replace trait defaults at
   [adapter.rs](../../crates/juggler-adapter/src/adapter.rs#L80).
4. **Flows (b)/(e) + verification:** new-project and launch-sheet
   entries; §8 checklist automated on Summary.

*Unresolved for implementers:* live cc-switch DB schema version
must be re-verified before S4 reads beyond existence; ZCode
per-account login probe needs the HOME-isolation experiment
re-run per [zcode.md](../research/zcode.md#L72);
adopt-in-place (flow f \*) requires its own ADR or stays cut.
# 04 — Daily Indicators & Right Inspector (Pass 2)

> **Scope:** the daily supervision surface only — what every
> session row/card shows at a glance, and what the right-hand
> inspector shows on selection. Onboarding, handover mechanics,
> and fleet transport are cited, not re-specified here.
> **Canonical input:** [expected-workflow.md](../internal/expected-workflow.md#L13)
> lines 13–18 — "lots of indicators … which provider it is,
> account, models currently running. Amount of quota left",
> "whole right section … stats, actions to manage, switch",
> "how much resources it is using, cpu, ram, did it spawn any
> sub tasks, sub processes", plus zombie/orphan tracking
> ([expected-workflow.md](../internal/expected-workflow.md#L17)).
> **Method:** full read of the canonical workflow, then targeted
> windows of the prior reports, `docs/SPEC.md`, `docs/PROPOSAL.md`,
> `docs/plan/EPICS.md`, and the live code they audit. Every claim
> below carries a repo `file:line`, a prior-report section, or
> an external URL. No runtime/browser test was run.

---

## 1. Terminology — adopt maintainer language, map, flag conflicts

The maintainer names three nouns
([expected-workflow.md](../internal/expected-workflow.md#L11)):
"we should call them providers, accounts, and instances of
the tool". This pass adopts that language in all user-visible
copy. The code/spec use different words for two of the three.

| Maintainer (UI) | Existing domain term | Where defined | Conflict? |
|---|---|---|---|
| Provider | Harness / `HarnessId` | [model.rs](../../crates/juggler-core/src/model.rs#L91) sketch in [SPEC.md](../SPEC.md#L91) | **Yes — rename in UI only** |
| Account | `Account` | [model.rs](../../crates/juggler-core/src/model.rs#L145) | No |
| Instance (tool instance) | `SessionRecord` + `InstanceKind` | [model.rs](../../crates/juggler-core/src/model.rs#L389), [model.rs](../../crates/juggler-core/src/model.rs#L325) | **Yes — overloaded** |
| *(no maintainer word)* | `Profile` (Account + overlay) | [model.rs](../../crates/juggler-core/src/model.rs#L173) | **Yes — invisible layer** |
| *(no maintainer word)* | Session (conversation) | ADR-0007 §1, [SPEC.md](../SPEC.md#L336) | **Yes — vs Instance** |
| Project | `ProjectRecord` | [model.rs](../../crates/juggler-core/src/model.rs#L371) | No |

**Conflict 1 — Provider vs Harness.** `HarnessId` values are
`"claude-code" | "codex" | "zcode"`
([SPEC.md](../SPEC.md#L91));
UI copy today says "Claude Code" as a static chip
(deep 01 §B-06). "Provider" in vendor docs usually means the
model/API provider (Anthropic/OpenAI), not the CLI. Resolution:
UI label = **Provider**, tooltip/subtitle = harness binary
(`claude`, `codex`, `zcode`); code keeps `HarnessId`. Never
show the word "harness" to users. Prior reports use "harness"
throughout (e.g. full review F5.5) — that word must not leak
into row/inspector copy.

**Conflict 2 — Instance vs Session (the load-bearing one).**
ADR-0007 §1–2 already decided: an *instance* is one running
occurrence (Terminal | Desktop); a *session* is the conversation
inside it. The shipped code collapses them: `SessionInfo` is
documented as "An instance/session row"
([SessionInfo.ts](../../ui/src/types/generated/SessionInfo.ts#L3))
and the Instances rail tab is Sessions filtered live/recent
through the same `SessionRow`
([WorkflowRail.tsx](../../ui/src/components/WorkflowRail.tsx#L295),
prior deep 01 §0.2, full review F2.2). Resolution for this
spec: **the rail row is the Instance; the inspector's History
tab is the Session.** One instance row may host 0..n past
sessions (resumes/forks). Do not add a second rail list until
that 1:n is real in the read model.

**Conflict 3 — Profile has no maintainer word.** A `Profile`
is Account + `SettingsOverlay{model, env}`
([model.rs](../../crates/juggler-core/src/model.rs#L161)).
Users think in Accounts; Profiles are launch presets. Resolution:
never a rail/inspector top-level noun. Show as a secondary
chip on the Instance (`Main · opus`) and as a chooser only
inside Launch / Hand over… actions. Prior deep 02 §9.2 (PF-04)
already treats profile fields as launch-form detail — extend
that: demote, don't promote.

**Rule for the rest of this file:** Provider = harness,
Account = login, Instance = running occurrence, Session =
conversation, Profile = launch preset (secondary only).

---

## 2. Second-pass verdict — what prior research missed

Prior work is strong on *actions* (deep 01/02 button audits)
and *states* (deep 02 §7). It under-weights the maintainer's
actual daily question in
[expected-workflow.md](../internal/expected-workflow.md#L13):
"at a glance, which of my many running things needs me, is
about to die on quota, or is eating the machine?" Five gaps:

**G1 — No per-row quota/cost/resource glance exists in prior
target designs.** Deep 02 §11.2 sketches one header line with
tokens + estimated cost, and full review UX.4 specifies rings
+ forecasts — but both live in the *panel/Overview*, not on
every rail row. The rail row today carries only label + raw
status string + a colour dot
([WorkflowRail.tsx](../../ui/src/components/WorkflowRail.tsx#L429)).
A user with 8 instances must click each one to learn quota
or burn. This pass moves quota bar, cost burn, and a CPU/RAM
sparkline *onto the row* (§3). Prior: deep 01 B-11, deep 02
WR-05 — extend, don't restate: they specify the status chip;
neither specifies quota/cost/sparkline on the row.

**G2 — Subprocess/subtask tracking is promised but never
given a UI.** [SPEC.md](../SPEC.md#L234)
requires the full process tree (subagents, node/electron
helpers, git/ripgrep children) attributed per session via
ppid-map, and the supervisor already exposes `descendants()`
([supervisor.rs](../../crates/juggler-supervisor/src/supervisor.rs#L341)).
No prior report puts a child-count badge on the row or a
Processes tab in the inspector. The maintainer's zombie
problem ([expected-workflow.md](../internal/expected-workflow.md#L17))
is therefore invisible until it hurts. This pass adds both
(§3 I-08, §5 Processes tab). Prior: full review F1.1/F5.1
discuss scale, not per-instance child visibility.

**G3 — Git state (branch/dirty) is mentioned once, never
specified as an indicator.** Deep 02 §6 notes "no branch/dirty
indicator exists anywhere" and parks worktree choice in a
launch popover (PP-05). The maintainer workflow is
project-first ([expected-workflow.md](../internal/expected-workflow.md#L5)):
two instances on the same repo on different branches is the
normal case, and a dirty tree is the top handover risk.
This pass makes branch + dirty count a first-class row and
inspector field (§3 I-09), sourced from `git status` on the
instance `cwd` ([model.rs](../../crates/juggler-core/src/model.rs#L405)).

**G4 — Prior "needs-attention" is a separate Inbox tab; the
row itself still doesn't rank.** Full review UX.3 and deep 01
§10 both add an Inbox as rail tab 1, sorted waiting-desc.
Correct — but insufficient: when the user is in Projects or
Sessions view, nothing on the row says "this one first".
This pass adds a per-row needs-attention badge with reason
text and makes it the default sort key in every list (§4),
so the Inbox becomes a filter, not a place you must visit.
Prior: full review F2.5 ("modelled but never emitted",
`model.rs:188-207` vs UI) — the emission gap is upstream;
this spec assumes emission (M2-5, [EPICS.md](../plan/EPICS.md#L79))
and specifies the rendering contract it must satisfy.

**G5 — Quality signals are invoked, never defined.** Prior
reports say "stuck detection ≥90% precision/recall" (SPEC
§9 metrics, [PROPOSAL.md](../PROPOSAL.md#L146))
and show "Stuck?" chips (deep 02 §7), but no report defines
tokens/min, error rate, or context-% — what is measured vs
inferred vs unknowable per Provider. Showing an undefined
"quality score" would repeat the fabricated-zero sin prior
reports rightly condemn (deep 01 B-14, `percent={0}` at
[App.tsx](../../ui/src/App.tsx#L513)).
§6 defines each signal with its honesty tier or refuses it.

**What prior got wrong (not just missed):** deep 01 B-02 /
deep 02 SH-02 propose *removing* Usage/Fleet from nav until
real. For the daily surface that is backwards: the *inspector*
Usage tab (§5) is where quota must live per selection, and it
can ship with honest "No data" states long before the global
Usage screen (M3, [EPICS.md](../plan/EPICS.md#L94)).
Removing the destination delays the per-instance habit.
Keep nav, but make every global number traceable to a row.

---

## 3. The indicator system — every row/card, every field

One row = one Instance. Fields I-01…I-11 in render order.
"Source" names the owning layer today; gaps name the task
that must add it. Refresh = how the *row* updates (inspector
rates in §5 may be faster). Stale rule (global): any value
older than 2× its refresh interval renders dimmed with a
`stale · Nm` caption — never silently as fresh (extends the
degraded-banner contract in deep 01 §8 / deep 02 §10).

| ID | Indicator (row rendering) | Data source today | Refresh | Empty / stale behaviour |
|---|---|---|---|---|
| I-01 | Provider icon + wordmark abbrev. (`◆ Claude`, `⬢ Codex`, `▲ ZCode`) | `SessionInfo.harness` ([SessionInfo.ts](../../ui/src/types/generated/SessionInfo.ts#L11)); icon set [icons.tsx](../../ui/src/components/icons.tsx#L22) (8 glyphs — Provider marks must be added) | Static per instance | Unknown harness → generic `◇ Tool` + text; never blank |
| I-02 | Account avatar: initials disc, per-Account hue + label (`WM` / `Work Max`) | `Account.label` ([model.rs](../../crates/juggler-core/src/model.rs#L151)); denormalised `ProfileInfo.account_label` ([ProfileInfo.ts](../../ui/src/types/generated/ProfileInfo.ts#L15)) — **not yet on `SessionInfo`; must be added** | Static; login flip live | Logged-out → avatar hatched + `Sign in` text chip (not colour-only) |
| I-03 | Model chip (`opus-4.8`, `gpt-5-codex`) | `ProfileInfo.model` ([ProfileInfo.ts](../../ui/src/types/generated/ProfileInfo.ts#L23)) via `SettingsOverlay.model` ([model.rs](../../crates/juggler-core/src/model.rs#L164)); **not on `SessionInfo` today** | On launch; on in-session `/model` change via transcript watch ([SPEC.md](../SPEC.md#L190)) | No overlay → `default model` (honest, not guessed name) |
| I-04 | State icon + text chip (9 states, §4 legend) | `SessionStatus` 9 variants ([model.rs](../../crates/juggler-core/src/model.rs#L188)); emission is the gap (full review F5.2) | Event-driven via `sessions_updated` ([bridge.ts](../../ui/src/lib/bridge.ts#L325)); target <300 ms ([EPICS.md](../plan/EPICS.md#L66)) | Unknown wire value → `Unknown` chip (parse returns `None`, [model.rs](../../crates/juggler-core/src/model.rs#L245)); never neutral-dot fallback |
| I-05 | Quota bar (account window) + `% left` + reset countdown (`17:00 · 2h10m`) | `QuotaWindow{used_percentage, resets_at}` ([model.rs](../../crates/juggler-core/src/model.rs#L279)); Claude statusline feed ([SPEC.md](../SPEC.md#L192)), ZCode computed ([SPEC.md](../SPEC.md#L210)), Codex estimated ([SPEC.md](../SPEC.md#L203)) | 30 s poll + on usage event; countdown ticks 1 min | No feed → dashed bar + `Quota: no data` (replaces fabricated `percent={0}`, deep 01 B-14); tier tag `Meas/Comp/Est` always adjacent ([model.rs](../../crates/juggler-core/src/model.rs#L288)) |
| I-06 | Cost burn (`$0.41` session, `~$3.20/h` rate) | Tokens `TokenCounts` ([model.rs](../../crates/juggler-core/src/model.rs#L263)) × pricing table — **pricing table does not exist** (full review F1.2: no USD view anywhere) | Tokens on transcript event; rate recomputed 30 s | No pricing for model → tokens only (`18.2k tok`), never a fabricated `$0.00`; estimates tagged `Est` |
| I-07 | CPU/RAM sparkline (60 s trailing, dual mini-area) + current (`12% · 1.1 GB`) | Supervisor process tree + sysinfo ([SPEC.md](../SPEC.md#L231): ~2 s trees) — **collector not yet implemented** (no sysinfo in supervisor today) | Sample 2 s, row repaint 5 s (sparkline), text 2 s | Desktop-kind / adopted orphans without tree → `—` + `resources unavailable`; flat-zero only if measured zero |
| I-08 | Subprocess/subtask count badge (`⑂ 4` children, `⧉ 2` subtasks) | `Supervisor::descendants()` ppid walk ([supervisor.rs](../../crates/juggler-supervisor/src/supervisor.rs#L341)); subtasks = adapter child-session refs (Claude subagent transcripts, [SPEC.md](../SPEC.md#L190)) — **count not yet surfaced to UI** | 5 s (with I-07 sample) | 0 → badge hidden (not `⑂ 0` noise); orphan-after-exit → badge persists amber with `left behind` (see Processes tab) |
| I-09 | Git branch + dirty (`⎇ main · 3●`) | `git status --porcelain=v2 --branch` on `SessionRecord.cwd` ([model.rs](../../crates/juggler-core/src/model.rs#L405)) — **not implemented anywhere today** (deep 02 §6) | On fs watch of `cwd/.git` + 15 s poll; after terminal commands (output event debounce 2 s) | Non-git cwd → field omitted entirely; detached HEAD → `⎇ detached@a1b2c3`; stale → branch dimmed |
| I-10 | Needs-attention badge (`! Needs you · 4m`, `⚠ Stuck?`, `⛔ Quota · resets 17:00`) | Derived from I-04 states WaitingUser/Stuck/QuotaBlocked/Failed (full review UX.3 sources) + waiting duration from `last_activity` ([model.rs](../../crates/juggler-core/src/model.rs#L411)) | Event-driven; waiting timer ticks 15 s | Mutually exclusive: highest-severity reason wins (§4 ranking); snoozed → badge collapses to `💤 12m` text |
| I-11 | Age + last-activity (`42m · active 10s ago`) | `started_at` / `last_activity` ([model.rs](../../crates/juggler-core/src/model.rs#L409)) | 15 s tick | Just started → `starting…`; no activity yet → `no output yet` |

**Card variant (Overview / project "On it now"):** same
I-01…I-11, two-line layout, sparkline widened to 5 min.
No field exists on card that is absent on row — Overview
must never show a number the rail cannot explain (the
`dashboard_stats` discard at [App.tsx](../../ui/src/App.tsx#L442)
is the anti-pattern this forbids).

---

## 4. Legend, redundancy, density, sorting & filtering

### 4.1 Legend — state (I-04/I-10), quota, confidence

Colour is **never** the only channel (full review UX.8 P1;
deep 01 §8). Every state = icon + text + colour together.
The legend lives in three places with identical glyphs:
rail footer `?` popover, inspector Overview header, and
the empty-Inbox "All clear" explainer.

| Glyph + text | Meaning (wire) | Colour token | Row action hint |
|---|---|---|---|
| `◌ Starting` | `starting` ([model.rs](../../crates/juggler-core/src/model.rs#L230)) | accent, pulsing ring | Stop |
| `● Running` | `running` ([:231](../../crates/juggler-core/src/model.rs#L231)) | live green + filled dot | Focus terminal |
| `! Needs you` | `waiting_user` ([:232](../../crates/juggler-core/src/model.rs#L232)) | warning amber + `!` badge | Focus terminal |
| `⚠ Stuck?` | `stuck` ([:233](../../crates/juggler-core/src/model.rs#L233)) — `?` is honesty (heuristic, [SPEC.md](../SPEC.md#L221)) | warning amber + dashed border | Inspect → Processes |
| `⛔ Quota blocked` | `quota_blocked` ([:234](../../crates/juggler-core/src/model.rs#L234)) | danger red + barred icon | Hand over… / Queue |
| `✓ Finished` | `finished` ([:235](../../crates/juggler-core/src/model.rs#L235)) | success + check | Resume |
| `■ Stopped` | `stopped` ([:236](../../crates/juggler-core/src/model.rs#L236)) | neutral grey + square | Resume |
| `✕ Failed` | `failed` ([:237](../../crates/juggler-core/src/model.rs#L237)) | danger red + `✕` | Recovery card |
| `⇄ Handed over` | `handed_over` ([:238](../../crates/juggler-core/src/model.rs#L238)) | accent + swap arrows | Jump to lineage |

Quota bar legend: fill = used %, hatch overlay = estimated
portion, `Meas/Comp/Est` text tag =
`Confidence` ([model.rs](../../crates/juggler-core/src/model.rs#L288)).
Threshold ticks at 70/90% carry text (`70%`, `90%`) — the
existing ring's colour-only thresholds
([primitives.tsx](../../ui/src/components/primitives.tsx#L178))
must gain `%` text + `role="img"` label (deep 01 B-14).
Account identity: hue is redundant with initials + label
text (I-02); colour-blind safe because hue is never load-
bearing. Sparkline legend: solid = RAM, outline = CPU%,
axis-free with `now` value in text (I-07).

### 4.2 Density modes (persisted per user, `aij.density`)

| Mode | Row height | Fields shown | For |
|---|---|---|---|
| Comfortable (default) | ~56 px, 3 lines | All I-01…I-11; sparkline 96 px wide | ≤8 instances, learning the tool |
| Compact | ~36 px, 2 lines | I-01 I-02 I-04 I-05(bar only) I-09 I-10; sparkline → single CPU%+RAM text | 9–20 instances (SPEC 20-session target, [SPEC.md](../SPEC.md#L20)) |
| Minimal / collapsed rail | 36 px icon rail (60 px, [WorkflowRail.tsx](../../ui/src/components/WorkflowRail.tsx#L110)) | Avatar disc + state glyph + `!` count; quota → 3-segment tick | Monitoring while coding |

Density never hides I-10 (needs-attention) or I-04 text in
Comfortable/Compact; Minimal keeps the glyph + badge count.
Switch: header segmented control + `⌘D` cycle; rows keep
`aria-label` with full field text in all modes.

### 4.3 Sorting & filtering (rail toolbar, above list)

Default sort = **Needs me first**, then quota-risk, then
recent activity. This replaces today's silent caps
(`slice(0,6/8/15)`, deep 02 WR-02/WR-05) — lists virtualise
past 50, search always present.

| Control | Options (in order) | Behaviour |
|---|---|---|
| Sort ▾ | Needs me (default) · Quota risk · Project · Provider · Account · Age · Name | Needs me = severity rank Failed > QuotaBlocked > NeedsYou(wait desc) > Stuck? > Running > rest; Quota risk = lowest `% left` × burn-rate (I-05×I-06), unknown-quota last with `no data` group header |
| Filter chips (multi) | `All` `! Needs me` `Running` `Ended` + Provider ▾ + Account ▾ + Project ▾ | Chips are toggles with counts (`Needs me (3)`); Provider/Account/Project are single-select dropdowns fed by I-01/I-02/project ([ProjectInfo](../../ui/src/types/generated/ProjectInfo.ts#L3)); combined filters AND across groups |
| Search `⌕` (`⌘K` focuses) | Free text over label, project, branch, model, account | Matches I-03/I-09 too (find "all on `main`"); `Esc` clears; empty result = `No instances match` + `Clear filters` button |
| Group-by ▾ | None · Project · Account · Provider | Group headers carry rollups (`3 running · 1 needs you · lowest quota 22%`); Needs-me sort applies inside groups |

Snooze/mute (from full review UX.3) lives on the I-10 badge
context menu: `Snooze 15m / 1h / Until reset`, `Mute this
instance`. Snoozed items sink below unsnoozed peers but
never disappear; badge on rail tab shows unsnoozed count
only. Fleet-ready: every filter key includes `machine`
([model.rs](../../crates/juggler-core/src/model.rs#L398))
from day one even though Fleet ships M6
([EPICS.md](../plan/EPICS.md#L134)).

---

## 5. Right inspector — full specification

Selection model: clicking any rail row (Instance, Project)
or global card (Provider, Account) replaces the inspector.
Header always shows: selection type icon, name, I-04 state
chip, and primary action. Six tabs for every selection type
— **Overview · Usage · History · Performance · Processes ·
Actions** — with per-type content below. Tab state persists
per selection type. Empty selection → inspector shows the
fleet-at-a-glance summary (counts by state, lowest-quota
Account, machine gauges from [SPEC.md](../SPEC.md#L345)),
never a blank panel. Stale rule from §3 applies per field;
each tab footer shows `Updated 3s ago · source: supervisor`.

Refresh-rate key used in all tables: **E** = event-driven
(<300 ms, [EPICS.md](../plan/EPICS.md#L66)),
**2s/5s/15s/30s/1m** = poll intervals aligned to
[SPEC.md](../SPEC.md#L231)
(staggered sysinfo), **M** = manual (button), **—** = static.

### 5.1 Selecting an Instance (the daily case)

Header: I-01 I-02 I-03, label, project chip (→ Project
inspector), `⎇ branch` (I-09), age (I-11). Primary:
terminal-kind → `Focus terminal`; desktop-kind → `Focus
window` ([SPEC.md](../SPEC.md#L347)).

| Tab | Stats / charts / actions (every element) | Source · refresh | Empty / stale |
|---|---|---|---|
| Overview | State chip + since; Provider/Account/Profile chips; model; project + cwd (copyable); native session id (`TranscriptRef.native_id`, [model.rs](../../crates/juggler-core/src/model.rs#L320)); quota bar + forecast (I-05); tokens + cost (I-06); CPU/RAM now (I-07); children (I-08); lineage (`⇄ from/to`, [SPEC.md](../SPEC.md#L302)) | Mixed E/2s/30s as §3 | No quota feed → dashed bar + `No data — arrives with statusline` ; no lineage → section omitted |
| Usage | Token breakdown (in/out/cache/r, [model.rs](../../crates/juggler-core/src/model.rs#L263)); tokens/min chart (§6); cost chart (uPlot, [SPEC.md](../SPEC.md#L349)); account-window contribution %; confidence tag per series | Transcript watch E; rollup 30s | Headless/no transcript yet → `No usage recorded yet`; Codex series hatched + `Estimated` |
| History | Session timeline for this instance (resumes, handovers, crashes); transcript viewer link; pack export preview ([SPEC.md](../SPEC.md#L294)) | Indexer ([SPEC.md](../SPEC.md#L41)) E on new session | Single session → one row, no timeline chrome; imported history badged `imported` ([model.rs](../../crates/juggler-core/src/model.rs#L360)) |
| Performance | CPU% + RSS charts (5m/1h); tokens/min; error-rate (§6); context-% gauge (§6, conditional); stuck-score spark (§6); footprint vs learned peak ([SPEC.md](../SPEC.md#L235)) | 2s sample, chart repaint 5s | <60s old → `Collecting…` skeleton; unavailable signal → named `Not measurable for this Provider` (never hidden) |
| Processes | Live tree table: PID, name, CPU%, RSS, age, `⑂` children; orphan section (below); per-row `Reveal` (no kill — Actions tab owns kills) | `descendants()` ([supervisor.rs](../../crates/juggler-supervisor/src/supervisor.rs#L341)) 5s | No children → `No subprocesses`; tree read fails → banner + Retry (M) |
| Actions | `Focus/Open terminal`, `Stop` (`⌘.`, deep 02 SP-04 rules), `Resume`/`Relaunch` (ended), `Hand over…` (`⌘⇧H`, wizard deep 02 §11.3), `Queue resume at reset` (QuotaBlocked only), `Export pack…`, `Archive`, `Delete…` (guarded, deep 02 §10) | Commands via bridge; busy spinner per button | Irrelevant actions disabled **with adjacent reason text** (deep 02 §10), never hidden; destructive = confirm popover + Undo toast |

### 5.2 Selecting a Session (conversation, from History/search)

Header: session label, owning Instance chip (→ 5.1), state,
started/ended. Tabs: **Overview** (turn count, tokens,
model(s) used, files touched from transcript IR
[SPEC.md](../SPEC.md#L185));
**Usage** (per-turn token/cost table, export CSV);
**History** (this session's lineage chain A→B→A,
[SPEC.md](../SPEC.md#L288));
**Performance** (tokens/min over session life, §6);
**Processes** (read-only: processes this session spawned,
from journal [journal.rs](../../crates/juggler-supervisor/src/journal.rs#L22) — ended sessions show final tree snapshot);
**Actions** (`Resume` primary `⌘R`, `Hand over…`, `Export
pack…`, `Fork/branch copy`, `Archive`, `Delete…`).
Refresh: E while live, — once ended (immutable + `Ended
HH:MM` stamp). Empty: unparsed transcript → `Transcript not
readable yet` + parser version, never zeroed stats.

### 5.3 Selecting a Provider (e.g. Claude Code)

Header: Provider icon + binary version + verified-range
badge (`discover()`, [SPEC.md](../SPEC.md#L158):
`verified 2.1.x · you run 2.1.296` or drift warning).
**Overview:** accounts count, running instances, aggregate
quota (worst account first), capabilities flags
([SPEC.md](../SPEC.md#L146)).
**Usage:** per-Account stacked burn (5h/weekly), all series
tier-tagged. **History:** adapter version history + session
counts by week. **Performance:** aggregate tokens/min,
error rate across accounts (§6 caveats doubled — cross-
account sums mix confidence tiers, label `mixed`).
**Processes:** all instances' trees grouped (read-only rollup
of 5.1 Processes). **Actions:** `Add account…` (wizard,
[SPEC.md](../SPEC.md#L409)),
`Check for Provider update` (M, never auto-update
[SPEC.md](../SPEC.md#L358)),
`Open Provider settings`. Refresh E/30s; empty (no accounts)
→ wizard CTA as the whole Overview.

### 5.4 Selecting an Account

Header: avatar (I-02), label, plan tier (`PlanTier`,
[model.rs](../../crates/juggler-core/src/model.rs#L120)),
login chip. **Overview:** quota rings (5h + weekly/credits,
existing `QuotaRing` [primitives.tsx](../../ui/src/components/primitives.tsx#L165)
+ `%` text fix), burn-down forecast + reset countdowns
([SPEC.md](../SPEC.md#L257)),
running instances on this account, usual-projects affinity
(full review F1.4). **Usage:** window timeline (uPlot),
per-session breakdown table, CSV export
([SPEC.md](../SPEC.md#L349)).
**History:** sessions by this account (grouped Today/
Yesterday/Earlier, deep 02 PP-07), handovers in/out.
**Performance:** tokens/min + cost rate trends; peak
footprint per Profile. **Processes:** instances' trees
(rollup). **Actions:** `Launch session…`, `Sign in /
Re-check login` (deep 02 PF-03 quartet), `New profile…`,
`Rename…`, `Delete account…` (danger, managed-dir-kept
note, deep 02 PF-08). Refresh: quota 30s, rest E/15s.
Empty: logged-out → Overview collapses to sign-in card;
quota unknown → rings dashed `— No data`.

### 5.5 Selecting a Project

Header: name, root (copy + Reveal), `⎇` summary across
instances, dirty total. **Overview:** "On it now" instance
card (full I-01…I-11), accounts-used history
([ProjectInfo.ts](../../ui/src/types/generated/ProjectInfo.ts#L19)),
instance/session counts. **Usage:** cost/tokens by Account
for this project. **History:** session timeline with lineage
links (deep 02 PP-07). **Performance:** per-instance resource
chart overlaid (who starved whom). **Processes:** all trees
in this project + orphan section. **Actions:** `Launch with
another account…` (rows ranked by headroom + `Usual` star,
deep 02 PP-05 fix), `Hand over project…`, `Open in editor`,
`Pin`, `Rename…`, `Archive project…`. Refresh E/15s.
Empty: missing dir → danger banner + `Relocate…` (deep 02
PP-02); no sessions yet → `Start first session` CTA.

**Processes-tab orphan contract (all selections, maintainer
zombie requirement, [expected-workflow.md](../internal/expected-workflow.md#L17)):**
when an Instance ends (any state except Stopped-clean), its
descendant set is snapshotted; survivors after 10 s grace
appear in an `Orphaned — left behind (N)` section with PID,
name, RSS, age-since-parent-exit, and parent label. Sources:
`descendants()` ([supervisor.rs](../../crates/juggler-supervisor/src/supervisor.rs#L341))
+ journal adoption ([supervisor.rs](../../crates/juggler-supervisor/src/supervisor.rs#L435),
[journal.rs](../../crates/juggler-supervisor/src/journal.rs#L151)).
Actions: `Verify owner` (re-check ppid chain, M), `Stop
process` (SIGTERM→SIGKILL ladder, [SPEC.md](../SPEC.md#L225),
confirm popover naming the PID), `Stop all orphans`
(confirm listing all). Auto-cleanup is **opt-in per Profile,
off by default**, grace 60 s, and only ever touches PIDs the
journal recorded as descendants — never a name-based kill
(compliance: "no killing other applications",
[PROPOSAL.md](../PROPOSAL.md#L47)).
Verification feedback: after any stop, row re-reads at 2 s
and shows `gone ✓` or `still alive — escalated to SIGKILL`;
failure keeps the row with error text. Empty: `No orphaned
processes` (a good empty state, cf. full review UX.3).

---

## 6. Quality & performance signals — honest measurability

No composite "health score". Each signal below stands alone
with its tier. If a Provider cannot supply it, the inspector
says so in words (§5 Performance empty rule).

| Signal | Definition (exact) | Measurable? | Source · refresh | Honesty label & failure mode |
|---|---|---|---|---|
| Tokens/min | Output+input tokens from transcript `usage` events in trailing 5 min ÷ 5 (Claude [SPEC.md](../SPEC.md#L193), Codex `token_usage_record` [SPEC.md](../SPEC.md#L202), ZCode `model_usage` [SPEC.md](../SPEC.md#L210)) | **Yes, all 3 Providers** — but bursty: show as 1-min bars + 5-min average, not a single twitchy number | Transcript watch E, recomputed 15 s | `Measured` (Claude/ZCode), `Measured tokens` for Codex; idle → `0 tok/min` is real, distinguish from `no data` (no transcript yet) |
| Cost/min & session cost | Tokens/min × per-model pricing; session = Σ per-turn | **Partially** — needs a maintained pricing table that does not exist (full review F1.2); subscription plans make "$ cost" a *value-equivalent*, not a bill | Pricing table (new, versioned in-repo) 30 s | Always `Est · value-equiv.`; hide $ entirely where pricing unknown (I-06 rule) |
| Error rate | (failed turns + `api_retry`/tool-error events) ÷ total turns, trailing session or 1 h | **Partially, Provider-dependent:** Claude headless exposes `system/api_retry` ([SPEC.md](../SPEC.md#L194)); interactive PTY and Codex/ZCode expose only transcript error patterns — parser-version-fragile ([SPEC.md](../SPEC.md#L184)) | Transcript parse E, rollup 30 s | Show numerator/denominator (`2 errors / 41 turns`), never a bare %; `Not measurable (interactive)` where no structured feed |
| Stuck score | **Not a score.** Three named heuristics from [SPEC.md](../SPEC.md#L220): (a) no output ∧ CPU < x% ∧ transcript mtime stale for N min; (b) waiting-on-permission > N min; (c) alive-but-no-progress. UI shows which rule tripped + its inputs (`no output 6m · CPU 0.3% · last write 6m ago`) | Heuristic only — thresholds user-tunable ([SPEC.md](../SPEC.md#L353)); precision/recall target ≥90% on fixtures ([EPICS.md](../plan/EPICS.md#L79)) is a *corpus* claim, not a per-session probability | Supervisor 15 s evaluation | Label `Stuck?` with `?` permanently (deep 02 §7); **refuse** a 0–100 score — it would fabricate precision the heuristic does not have |
| Context % | Tokens in current context ÷ model context limit | **Provider-dependent, often unknowable:** measurable only where transcript/statusline reports context tokens (Claude statusline JSON, [SPEC.md](../SPEC.md#L192)); Codex/ZCode compact/summarise internally with no public gauge | Statusline E where present | Where absent: gauge replaced by `Context: not reported by {Provider}` + cumulative session tokens (labelled cumulative, not context). **Never estimate from cumulative tokens** — compaction makes that number wrong, not approximate |
| CPU/RAM (I-07) | Process-tree RSS Σ + CPU% (ppid-attributed, [SPEC.md](../SPEC.md#L234)) | **Yes** via sysinfo staggered refresh ([SPEC.md](../SPEC.md#L231)); overhead budget <1% CPU ([SPEC.md](../SPEC.md#L24)) | 2 s sample | `Measured`; short-lived children may be missed between samples — footnote `2s sampling` on Processes tab |
| Quota forecast | `time-to-empty = % left ÷ current burn rate` ([SPEC.md](../SPEC.md#L257)) | Yes where I-05+I-06 exist; meaningless at idle (÷0) | 30 s | Show range (`~30–50m at current rate`), tier-tagged; idle → `not burning` instead of ∞ |

External calibration: ccusage proves local-file token
accounting is viable across ~20 tools
([PROPOSAL.md](../PROPOSAL.md#L123));
Opcode ships the cost-by-model dashboard users will compare
against ([github.com/winfunc/opcode](https://github.com/winfunc/opcode),
cited in deep 01 §8). Both set the bar at *labelled*
numbers — the differentiator is the honesty tier, not the
chart library (uPlot, [SPEC.md](../SPEC.md#L83)).

---

## 7. ASCII wireframes — rail + inspector

### 7.1 Rail (Comfortable density, Needs-me sort, §4)

```
┌─ AI Juggler ──────────────── ⌘B ┐
│ [Projects][Instances][Sessions] │  tabs (deep 01 B-08 fix)
│ ⌕ search…            ⌘K  [Sort▾]│
│ (All)(! Needs me 2)(Running)▾   │  filter chips + counts §4.3
│─────────────────────────────────│
│ ◆Claude [WM] opus-4.8  ●Running │  I-01 I-02 I-03 I-04
│   fix-auth · ⎇main·3●  42m      │  I-09 I-11
│   Quota ██████░░ 78% left Meas  │  I-05 + tier
│   18.2k tok · $0.41 Est ·⑂4 ⧉2  │  I-06 I-08
│   ▁▃▅▂▅ 12%·1.1GB  active 10s   │  I-07 spark + text
│─────────────────────────────────│
│ ◆Claude [PS] opus-4.8 !Needs you│  I-10 badge (amber, text)
│   review-pr · ⎇feat/x  wait 4m  │
│   Quota ████░░░░ 55% · ▁▂▁ 3%   │
│─────────────────────────────────│
│ ⬢Codex [WM] gpt-5 ⛔Quota 17:00 │  I-10 danger + reset
│   spike · ⎇main  1h05m          │
│   Quota █░░░░░░░ 8% left Est    │
│ [＋ New session ⌘N]             │
│ A 22% · B 81% · !2   ? legend   │  footer: quota glance
└─────────────────────────────────┘  (deep 01 B-05 redesign)
```

Collapsed (Minimal): avatar discs with state glyph +
`!` badge only; tooltip carries the full I-01…I-11 text.

### 7.2 Right inspector — Instance, Overview tab (§5.1)

```
┌─ ● fix-auth [Running][terminal] ─────── [Focus ⌘⏎][⋯]┐
│ ◆Claude [WM]Work Max·Main·opus-4.8  my-app/ ⎇main·3● │
│ Overview│Usage│History│Performance│Processes│Actions │
│───────────────────────────────────────────────────────│
│ State Running 42m · active 10s ago   sess 9f3… ⧉     │
│ Quota (Work Max, 5h) ██████░░ 78% left · Meas        │
│   resets 17:00 (2h10m) · exhausts ~3h at current rate│
│ Tokens 18.2k (in 12k/out 6k) · $0.41 Est             │
│ CPU ▁▃▅▂ 12% · RAM 1.1 GB · ⑂4 children · ⧉2 tasks  │
│ Lineage: ← sess-41 (Personal)                        │
│ [Hand over… ⌘⇧H] [Stop ⌘.]        Updated 3s ago     │
└───────────────────────────────────────────────────────┘
```

### 7.3 Inspector — Processes tab with orphans (§5.1/§5.5)

```
│ Processes (5s)                         [Verify owner] │
│ PID    Process          CPU   RAM    Age   ⑂        │
│ 41201  claude           11%   890MB  42m    4        │
│ 41233  └ node (mcp)      1%    96MB  40m    0        │
│ 41302  └ rg (search)     4%   120MB   12s   0        │
│ ⚠ Orphaned — left behind (2)        since exit 18m  │
│ 39871  node dev-server   0%   210MB  18m  [Stop…]    │
│ 39890  └ vite            0%    88MB  18m  [Stop…]    │
│        [Stop all orphans…]  (confirm lists PIDs)     │
```

Acceptance for both wireframes: keyboard-only path
`⌘K → instance → inspector tab (⌘1…6 within inspector) →
action` without mouse; every number above reachable as
text (screen reader), no colour-only meaning (§4.1);
at Compact density the inspector is unchanged (density
is a rail-only control, §4.2).

*End of pass-2 §04. Siblings in `docs/review/pass2/` cover
onboarding, handover, and fleet against the same canonical
workflow; shared contracts: §1 terminology, §4 legend.*
# 05 — Process Trees, Zombies, Orphans and the Cleanup Centre (Pass 2)

> Second-pass review. First pass is cited, not restated: see
> `AI-Juggler-Maximum-Depth-Report-and-Proposal.md` §5 (supervisor FMEA
> F-1..F-10), §6 (scale model), §8 (target architecture / ADR-0010), and
> `docs/review/deep/08-architecture-data-supervisor-security.md` §5; UX
> cross-check `docs/review/full-competitive-ux-review.md` §Q5/F5.1.
> Canonical demand: `docs/internal/expected-workflow.md:15,17-18`. Spec anchor:
> `docs/SPEC.md:216-252` (§5.1–§5.3) and `docs/plan/EPICS.md:46,80`
> (M0-5, M2-6, M2-7). Code read in full:
> `crates/juggler-supervisor/src/supervisor.rs` (585 lines),
> `journal.rs` (216), `lifecycle.rs` (53), `error.rs` (39),
> `ring.rs` (129), `lib.rs` (24), plus callers in
> `crates/juggler-app/src/state.rs` and `commands.rs`.

## 0. Verdict in one paragraph

The maintainer names this as a headline problem, not hygiene:
tools “crash, exist, and their subprocesses, cli commands, terminals
and dev local services they started they end up running long after
the session closed, they end up being zombie processes” and the tool
“should have super easy way to track that, offer auomatic and manual
clean ups, all with verifications” (`docs/internal/expected-workflow.md:17-18`).
Pass 1 correctly found the kill-path defect (F-9) and the adoption
identity defect (F-2), but treated both as supervisor bugs to patch.
That under-weights the demand in three ways. First, the expected
workflow asks for a *product surface* — per-session resource and
sub-process visibility (“for each tool, session, we should [show] how
much resources it is using, cpu, ram, did it spawn any sub tasks, sub
processes, and allow to track those”, `docs/internal/expected-workflow.md:15`)
plus a cleanup experience — and no pass-1 section designs that surface.
Second, pass 1’s F-5 verdict (“our zombies: none”, Maximum-Depth §5)
answers the narrow Unix-reaping question and thereby closes the topic
the maintainer actually raised, which is mostly *orphaned live
processes*, not `Z`-state entries. Third, the two mechanisms that make
cleanup safe — process-group ownership at spawn time and a persistent
spawn ledger for descendants — do not exist in code today, so the
F-9 fix (“TERM tree → grace → KILL tree”) cannot be implemented
safely by editing `stop()` alone. This file designs the missing
system: ledger, identity, detection, port map, Cleanup Centre,
verify-before-kill, tiered auto-cleanup, audit, and crash recovery.

## 1. Terminology: maintainer language over code language

Maintainer vocabulary is explicit: “we should call them providers,
accounts, and instances of the tool” (`docs/internal/expected-workflow.md:11`),
with daily indicators “which provider it is, account, models currently
running” (`docs/internal/expected-workflow.md:13`). The code speaks a different
dialect. Mapping (adopt maintainer terms in UI; keep code names only
where renaming is churn):

| Maintainer term | Code term today (evidence) | Conflict / action |
|---|---|---|
| Provider | `HarnessId` “a harness kind, e.g. claude-code” (`crates/juggler-core/src/model.rs:44-47`); SPEC/PROPOSAL say “harness” (`docs/SPEC.md:12`, `docs/PROPOSAL.md:12`) | UI must say Provider; `HarnessId` stays as the internal id. Never show “harness” to users. |
| Account | `AccountId` / `Account` (`crates/juggler-core/src/model.rs:48-51`) | Aligned. Keep. |
| Instance (tool instance) | `SessionRecord` + `InstanceKind { Terminal, Desktop }` (`crates/juggler-core/src/model.rs:323-333,386-416`); ADR-0007 already decided “what the code called a launched session is, in product language, an instance” (`docs/adr/0007-instance-centric-information-architecture.md:28-33`) | Partially aligned in docs, not in types. Cleanup UI must group by Instance; the ledger key stays `SessionId` internally until a rename epic. Flag: `stop_session` command name (`crates/juggler-app/src/commands.rs:733-743`) exposes the old word across the Tauri bridge. |
| Session (conversation) | Also `SessionRecord`/`SessionId` — the same type carries both instance and conversation (`crates/juggler-core/src/model.rs:391-416`) | **Real conflict.** ADR-0007 separates instance (occurrence) from session (conversation) in prose but the struct merges them. Cleanup attributes processes to the *instance* (the spawner); handover attributes transcripts to the *session*. The ledger below therefore records both `session_id` (current key) and a future `instance_id`, so the split needs no re-journaling. |
| Profile | `Profile` = account + settings overlay (`crates/juggler-core/src/model.rs:170-184`) | No maintainer word. Keep as an Account-level launch preset; never present Profile as a peer of Provider/Account/Instance in cleanup UI. |
| (none) | Process / child / descendant (`crates/juggler-supervisor/src/supervisor.rs:341-356`) | New UI words needed: **Spawned process** (direct child), **Helper** (any descendant), **Orphan** (§2). Introduce in `juggler-core`, not as UI-only strings. |

Rule for the rest of this file: **Provider / Account / Instance**
are user-facing; `HarnessId / AccountId / SessionId` are cited only
as code evidence.

## 2. A precise taxonomy (pass 1 conflated three different things)

Pass 1’s F-5 (Maximum-Depth §5, `deep/08` §5) is correct that direct
children are waited (`crates/juggler-supervisor/src/supervisor.rs:157-171,242-259`)
and that `alive_pids` excludes `Zombie|Dead`
(`crates/juggler-supervisor/src/journal.rs:199-216`). It then
concludes “None; keep zombie filter”. That is the under-weighting:
the maintainer’s “zombie processes” (`docs/internal/expected-workflow.md:17`)
is the colloquial umbrella for at least five mechanically distinct
states, each needing different detection and different UX:

| # | State | Mechanism | Dangerous? | Correct verb |
|---|---|---|---|---|
| T1 | True zombie (`Z`) | Child exited, parent has not `wait()`ed; holds only a PID-table slot | No CPU/RAM; blocks PID reuse accounting | Reap (parent) or ignore if parent is `init`/launchd |
| T2 | Orphaned live descendant | Instance root exited/was killed; descendant reparented to PID 1, keeps running (dev server, MCP server, watcher) | **Yes** — RAM/CPU/ports/quota | Verify → terminate (this file’s core) |
| T3 | Detached-by-design survivor | Juggler itself exited; supervised roots survive intentionally (`crates/juggler-supervisor/src/supervisor.rs:47-51`) | No — this is SPEC §5.3 behaviour (`docs/SPEC.md:251`) | Re-adopt, never auto-kill |
| T4 | Leaked direct child of a live instance | Instance claims `Stopped`/`Finished` but its root still runs (stop failed silently) | Yes | Escalate ladder, surface as failed stop |
| T5 | Stranger on a recycled PID | PID reused after crash; journal still points at it (F-2) | **Critical if killed** | Never kill; identity check must fail closed |

Two consequences follow. (a) The UI must never label T2/T3 rows
“zombie”: users will learn the wrong mental model and fear T3
survivors that are working as designed. Suggested labels:
`Still running after its Instance closed` (T2), `Running — re-adopted`
(T3), `Unverifiable — will not be touched` (T5). (b) F-5’s “residual:
piped grandchildren reparented to init — out of scope, document it”
is precisely T2, which the expected workflow puts *in* scope
(`docs/internal/expected-workflow.md:17-18`). Pass 2 reverses that scoping:
T2 is the product; T1 is the footnote.

## 3. What pass 1 got right, and what it missed or under-weighted

Cited so this file extends rather than repeats (Maximum-Depth §5
table; identical text at `deep/08` §5):

- **Right — F-9 descendant leak.** Force-kill touches only the direct
  child (`crates/juggler-supervisor/src/supervisor.rs:149-151,234-236`);
  `descendants()` (`supervisor.rs:347-356`) is query-only and no caller
  in the kill path uses it — the only production `stop` callers pass a
  bare session id (`crates/juggler-app/src/commands.rs:740`,
  `crates/juggler-app/src/terminal.rs:200`). Agree, and §9 hardens it.
- **Right — F-2 PID reuse.** Adoption trusts PID liveness alone
  (`crates/juggler-supervisor/src/journal.rs:188-196`); `program` is
  stored (`journal.rs:35-40`) but never compared. Agree; §6.2 makes the
  check fail-closed and extends it from roots to every helper.
- **Right — F-3 adopted orphans are uncontrollable.**
  `alive_adopted_sessions` returns ids only
  (`crates/juggler-supervisor/src/supervisor.rs:432-438`) and no
  `ChildHandle` is rebuilt, so `stop` returns `UnknownSession`
  (`supervisor.rs:296-303`). Agree; §11.2 specifies the adopted handle.
- **Under-weighted — F-5 scoping.** See §2: T2 dismissed as residual.
- **Missed — no process-group is ever created.** A repo-wide search
  finds no `setpgid`/`setsid`/`process_group`/`killpg` anywhere under
  `crates/`. Piped spawns use bare `std::process::Command`
  (`crates/juggler-supervisor/src/supervisor.rs:129-140`); PTY spawns
  rely on the PTY session (`supervisor.rs:194-213`) without recording
  the PGID. Without a spawn-time PGID (or equivalent job object),
  “kill the tree” degrades to a racy ppid-walk snapshot — a helper
  can fork between the walk and the signal and escape. This is the
  load-bearing omission behind F-9.
- **Missed — descendants are invisible in the journal.** The journal
  records only root spawns/exits
  (`crates/juggler-supervisor/src/journal.rs:34-48`:
  `Spawned{session,pid,program,at}`, `Exited{session,pid,code,signal,at}`).
  Once the root exits, Juggler retains *no record* that a vite server
  or MCP helper ever belonged to it. Post-exit orphan detection is
  therefore impossible from current data, at any refresh rate.
- **Missed — no port dimension at all.** No listener/port enumeration
  exists in the supervisor or app crates (no `TcpListener`/LISTEN/`lsof`
  usage; `crates/juggler-supervisor/Cargo.toml:19` has `sysinfo` only).
  Yet the maintainer’s examples are port-holders: “dev local services
  they started” (`docs/internal/expected-workflow.md:17`). A cleanup list that
  cannot say “PID 8123 holds :5173” cannot earn user trust.
- **Missed — no resource attribution to show.** SPEC §5.2 already
  promises per-session tree RSS/CPU “attributed to the session even as
  children come and go (ppid-map tracking)” (`docs/SPEC.md:234`) and
  an RSS-anomaly alert (`docs/SPEC.md:243`); EPICS carries it as M2-6
  (`docs/plan/EPICS.md:80`). Code has neither: `descendants_of`
  (`crates/juggler-supervisor/src/supervisor.rs:557-579`) returns bare
  `Vec<u32>` with no RSS/CPU, and each call builds a fresh
  `System::new()` full-process refresh — the cost pass 1 flagged at
  Maximum-Depth §6.2(3). The gap is a data model, not a widget.
- **Missed — `children` map grows forever.** Insert-only
  (`crates/juggler-supervisor/src/supervisor.rs:285-287`); `stop`
  never removes, exit never evicts (F-6). A Cleanup Centre that scans
  this map inherits the leak as phantom “Running” rows.
- **Wrong-by-omission — SPEC’s own promise is untestable today.**
  SPEC §5.1 promises “zombie/orphan detection on app restart”
  (`docs/SPEC.md:224`) and §5.3 re-adoption “by PID + log
  correlation” (`docs/SPEC.md:251`). The only restart test spawns a
  lone `sleep 300` with no children
  (`crates/juggler-supervisor/tests/supervisor.rs:118-151`), and the
  only tree test asserts descendants exist but never that they die
  (`tests/supervisor.rs:153-173` — it stops the root and waits only
  for the root’s exit event). The fixtures in SPEC §10 (“fake harness
  spawning memory/CPU-hog children”, `docs/SPEC.md:439`) do not exist.

## 4. Design principle: ownership must be created at spawn, not inferred later

Every safe cleanup decision reduces to one question — *did an Instance
we manage create this process?* Today that is inferred after the fact
from a ppid walk (`crates/juggler-supervisor/src/supervisor.rs:557-579`)
that is correct only for one instant and only while ancestors live.
Three spawn-time mechanisms replace inference with ownership:

1. **Process group per Instance (Unix).** Each root is spawned as a
   group leader (`setsid` for piped children via `Command::pre_exec`;
   PTY children already get a controlling terminal — record the PGID
   explicitly instead of assuming it). Helpers that do not themselves
   call `setsid` stay in the group, so `killpg` reaches the dev server
   that double-forked away from the ppid chain. This is the standard
   Unix ownership primitive; Juggler currently creates none (§3).
2. **Job Object per Instance (Windows).** `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`
   gives the same guarantee with OS enforcement. Deferred to the
   Windows bring-up (EPICS M7-5, `docs/plan/EPICS.md:80` rows) but the
   trait in §12 must leave the seam now.
3. **Spawn ledger (all platforms).** A persistent record of every
   root *and every observed helper*, written before it is needed.
   The ledger is what survives the root, the app, and the reboot —
   the three moments ppid inference fails.

macOS note: there is no Linux-style cgroup to lean on; PGID + ledger
+ endpoint verification is the portable trio. Where a helper
deliberately daemonises (`setsid` + double-fork, e.g. some dev
servers), PGID escapes too — the ledger’s ppid snapshots (§5) plus
port/cwd corroboration (§7) are the fallback, and the UI must say
`escaped its process group` honestly rather than claim containment.

## 5. The spawn ledger (new persistent state)

The current journal (`crates/juggler-supervisor/src/journal.rs:34-48`)
is an append-only root log. It stays as the crash-recovery log, but
cleanup needs a queryable ledger. Proposal: a `process_ledger` table
in the existing SQLite store (WAL, per SPEC §3 storage), written by
the supervisor through a narrow trait so `juggler-supervisor` stays
storage-agnostic. Columns (Rust type in §12):

- Identity: `pid`, `pgid`, `started_at` (process start time from the
  OS, not wall-clock spawn time — the F-2 fix), `exe_path`,
  `cmdline_hash` + truncated `cmdline` (display), `boot_id` (changes
  every reboot; kills stale-PID false positives outright).
- Attribution: `session_id` (Instance), `provider` (`HarnessId`),
  `account_id`, `project_id`, `root_pid`, `parent_pid` at first
  observation, `depth` (1 = direct child), `first_seen`, `last_seen`.
- Role guess: `role` ∈ `Harness | Subagent | DevServer | McpServer |
  TerminalShell | Watcher | BuildTool | Unknown` — inferred from exe/
  cmdline/port heuristics, always shown as a guess, never as fact.
- Lifecycle: `state` ∈ `Running | Exited | Orphaned | Adopted |
  Unverifiable`, `exit_kind`, `reaped_by_us`.
- Evidence: `listens_on` (ports), `cwd`, peak RSS/CPU (§8).

Write discipline matters more than schema. Three rules fix F-7’s
ordering bug (spawn precedes journal at
`crates/juggler-supervisor/src/supervisor.rs:107-114`, so a journal
failure orphans an unrecorded process): (1) write an `Intent` row
(state `Starting`, no PID yet) *before* spawn; (2) patch in PID/PGID/
start-time immediately after; (3) never fail the spawn because the
ledger failed — degrade to `ledger_degraded` health flag instead.
Helper rows are upserted by the reconciler (§6) at most once per
2 s tick per Instance, batched in one transaction.

Retention: exited-helper rows are kept 7 days (audit + “what did I
kill?” history), then pruned by the same job that prunes journal
entries. Rows for processes still running are never pruned.

## 6. Liveness and orphan detection

### 6.1 The reconciler loop

One shared `sysinfo::System` refreshed on the SPEC §5.2 cadence
(“~2 s for supervised process trees”, `docs/SPEC.md:231`) replaces
today’s per-call `System::new()` in both
`crates/juggler-supervisor/src/supervisor.rs:557-579` and
`crates/juggler-supervisor/src/journal.rs:200-216` (the cost flagged
at Maximum-Depth §6.2(3)). Each tick the reconciler:

1. Refreshes once; builds the ppid map in-memory (no allocation of a
   fresh map per query as `descendants_of` does today).
2. For each live Instance root: walks descendants, upserts helper
   rows, refreshes RSS/CPU/ports, detects role changes.
3. For each root that exited since last tick: marks helpers whose
   ppid is now 1 (or whose ancestor chain no longer reaches a live
   Juggler-owned root) as `Orphaned` (T2) — *after* a grace window
   (§6.3), because well-behaved helpers exit milliseconds after their
   parent and must not flicker into the Cleanup Centre.
4. For each ledger row in `Running|Orphaned|Adopted`: re-verifies
   identity (§6.2). A failed check flips the row to `Unverifiable`
   (T5) and permanently removes it from every kill path.
5. Emits deltas as lifecycle events; the UI never polls `ps` itself.

### 6.2 Identity verification (fail closed)

A PID is a name, not an identity. Before *any* signal, and before a
row is even shown as killable, three facts must agree with the
ledger: (a) OS process start-time equals the recorded start-time;
(b) resolved exe path equals the recorded exe (symlinks resolved);
(c) `boot_id` matches the current boot. This extends pass 1’s F-2
fix (Maximum-Depth §5: “verify /proc/ps exe path + start-time”) from
adoption-time roots to every helper at kill-time, because helpers are
exactly where PID reuse bites during a long session. Cost: one
`stat`/`proc_pidinfo` per candidate per tick — negligible beside the
2 s full refresh SPEC already budgets (`docs/SPEC.md:231`).
External background on why start-time matters: PID reuse is a
documented hazard of PID-only tracking —
[man7.org proc(5) on PID reuse](https://man7.org/linux/man-pages/man5/proc.5.html).

### 6.3 Orphan grace and flap control

Defaults (all user-tunable, stored per Provider): `orphan_grace =
30 s` after root exit before a helper is listed; `hog_window = 60 s`
sustained breach before a hog alert (§8); `adopted_idle = 24 h`
before a T3 survivor is *suggested* (never auto-cleaned, §10). A
helper that exits during grace is journaled as a normal exit and
never surfaces. Two consecutive ticks must agree a process is
orphaned — single-tick sightings are hidden, which also absorbs
ppid-map races during fork storms (e.g. `cargo build` spawning
dozens of short-lived `rustc` helpers).

## 7. Port occupancy map

Ports are the evidence users already understand: “something is on
:3000”. The map joins three sources per reconciler tick: the ledger
(which PID belongs to which Instance), the OS listener table, and a
curated well-known-dev-port hint list (3000, 4173, 5173, 8000, 8080,
9229 — hints only, never identity). Implementation: `sysinfo`
0.39 (`crates/juggler-supervisor/Cargo.toml:19`) does not expose
listeners portably, so use `netstat2` on macOS/Linux or
`GetExtendedTcpTable` on Windows behind a `PortProbe` trait (§12);
fall back to “ports unknown on this platform” rather than shelling
out to `lsof` (slow, permission-flaky, and unparsable under load).

Each port row shows: port, owning PID → Instance → Provider/Account/
Project, listener age, and a collision badge when the owner is *not*
the Instance the user is about to launch into that project. Two
user-facing wins follow directly: launch can warn “:5173 is already
held by an orphan from Tuesday’s Instance” *before* vite fails with
`EADDRINUSE`, and the Cleanup Centre can offer “free this port” as a
goal-oriented action that resolves to a verified PID kill (§9).
Processes owned by the user outside Juggler (their own `npm run dev`)
appear in the map as `Not managed by Juggler` and are display-only —
seeing them prevents the classic mis-kill, and §13-F3 requires it.

## 8. Resource-hog alerts (attribution before alarm)

SPEC §5.2 promises tree RSS/CPU per session (`docs/SPEC.md:234`),
anomaly alerts at “sudden ≥ 3× growth” (`docs/SPEC.md:243`), and the
historical `footprint_stats` advisor (`docs/SPEC.md:235-237`); none
exists in code (§3). Pass 2 scopes the *minimum* that makes cleanup
decisions informed, deferring the full advisor to M2-6
(`docs/plan/EPICS.md:80`):

- Per-helper and per-Instance rollups each tick: RSS, CPU% (delta over
  the 2 s window, respecting `sysinfo`’s two-sample CPU requirement),
  cumulative CPU time. Stored as peaks in the ledger (§5).
- Hog rules (sustained over `hog_window`, §6.3): Instance tree RSS
  above the per-Provider learned p95, or any single helper above an
  absolute floor (default 2 GiB), or CPU above 90% of one core for
  the full window. A hog alert names the top helper, not just the
  Instance — “`node (vite)` under Instance X holds 3.1 GiB” is
  actionable; “Instance X is heavy” is not.
- Alerts are passive suggestions with a staged action, per the
  automation invariant: the app “prepares; the user executes”
  (`docs/SPEC.md:26`). A hog alert may pre-fill a Cleanup Centre
  selection; it may never signal a process (§10).
- Idle-orphan nudge: a T2 orphan idle (CPU ≈ 0, no listened-port
  traffic signal available portably — so CPU+age only) for 1 h raises
  one low-severity suggestion. No repeated nagging: once per orphan.

## 9. The Cleanup Centre (product surface)

A new global surface (top-right global menus per ADR-0007’s
“global / setup mode”, `docs/adr/0007-…md:20-23`; not a rail peer of
Projects/Instances/Sessions). It is the answer to “super easy way to
track that” (`docs/internal/expected-workflow.md:18`) and to the per-Instance
resource demand (`docs/internal/expected-workflow.md:15`).

### 9.1 List model

One row per *process* (not per Instance), grouped by Instance with a
collapsed “healthy tree” default. Columns: state badge (T1–T5 labels
from §2), role guess, exe/cmdline (truncated, expandable), PID/PGID,
age, RSS/CPU (live), ports, Provider · Account · Project, and the
verification badge (§9.2). Filters: `Needs attention` (default:
Orphaned + hogs + port collisions), `All managed`, `Re-adopted`,
`History (7 days)`. Every row carries “Why does Juggler think this is
mine?” expanding to the evidence chain (spawned-by root, first-seen
ppid, PGID match, exe/start-time check). Empty state is a success
state: “Nothing left behind — N Instances running clean.”

The Instance panel (ADR-0007 Decision 2: “status, account/profile,
kind, age, resources, actions”, `docs/adr/0007-…md:35-40`) embeds the
same rows filtered to that Instance, satisfying
`docs/internal/expected-workflow.md:14-15` without a second data path.

### 9.2 Verify-before-kill flow (no exceptions)

Killing is a three-step staged action; there is no one-click kill
anywhere, including the Instance panel and hog alerts:

1. **Verify (automatic, on selection).** Re-run §6.2 identity checks
   *now* (not from the last tick), re-read exe/cmdline/cwd/ports, and
   show a confirmation sheet: full cmdline, cwd, ports that will be
   freed, parent Instance and its state, and what else shares the
   PGID. If verification fails, the sheet becomes an explanation
   (“This PID now belongs to a different program — Juggler will not
   touch it”) with the kill button removed, not disabled-with-tooltip.
2. **Choose scope.** `This process only` / `This process and its
   helpers` (PGID ladder) / `Whole Instance tree`. Default is the
   narrowest that covers the user’s stated goal (“free :5173”
   defaults to process-only).
3. **Confirm with consequence text.** The button names the outcome:
   `Terminate vite (PID 8123) and free :5173`. Destructive confirms
   follow the house pattern; there is no “Don’t ask again” for kills.

### 9.3 Manual TERM → KILL ladder UX

Extends the existing root-only ladder
(`crates/juggler-supervisor/src/supervisor.rs:290-318`: SIGTERM, poll
10 ms, force-kill) to the verified scope: TERM the PGID (or the single
PID for process-only), show a live countdown with per-process exit
ticks as each helper dies, then KILL survivors only after the grace
(default 5 s, matching today’s `stop_session` grace at
`crates/juggler-app/src/commands.rs:740`). The user can cancel during
grace; a helper that ignores TERM is labelled `Ignored SIGTERM` before
any KILL is offered — surprise SIGKILLs are how data is lost (a dev
server is cheap to kill; a database helper is not). On Windows the
ladder is `TerminateProcess` only, and the UI must say so.

After the ladder, the sheet reports per-process outcomes (exited on
TERM / needed KILL / already gone / refused) and writes the audit
record (§11). “Already gone” is a success, not an error.

## 10. Safe auto-cleanup policy (opt-in tiers)

The expected workflow asks for “auomatic and manual clean ups”
(`docs/internal/expected-workflow.md:18`), but SPEC’s automation invariant
forbids state-changing actions without a click, with one narrow
exception (opt-in per-profile auto-restart, `docs/SPEC.md:26`).
Auto-cleanup therefore needs the same treatment: an explicit,
narrowly-scoped, off-by-default opt-in, recorded as a SPEC amendment
alongside the auto-restart exception — not a quiet settings toggle.
Pass 1 never surfaced this conflict; resolving it is a precondition,
not a detail. Proposed tiers:

| Tier | Default | What it may do without a click | Hard exclusions |
|---|---|---|---|
| 0 Observe | **On** | Detect, list, alert, pre-stage cleanup sheets | Never signals anything |
| 1 Tidy verified orphans | Off (opt-in per Provider) | After `orphan_grace` + identity re-verified at kill time: TERM→KILL helpers whose Instance root exited *cleanly* (code 0) and which hold no unsaved-work role (`DevServer`, `Watcher` only) | Never roots; never `McpServer`/shell/DB roles; never after a crash exit |
| 2 Tidy after crash | Off (separate opt-in) | As Tier 1, but for crash-exited Instances, after 5 min and one user-visible notification with Undo window | Never T3 adopted survivors; never `Unverifiable`; never within `adopted_idle` |
| 3 Hog shed | **Never automatic** | — (alert + staged sheet only, §8) | A busy process is not a dead process; no tier may kill for resource use alone |

Invariants for all tiers: identity is re-verified in the same tick
as the signal (§6.2); every automatic action writes the §11 audit
record *before* signalling; a global “Pause automatic cleanup”
kill-switch lives in Settings and in the Cleanup Centre header;
any verification failure silently demotes the candidate to Tier 0
(list-only). Tier 1/2 are suspended for 24 h after any Juggler crash
recovery (§11.2) until the user re-confirms — the moment identity
data is least trustworthy is right after a crash.

## 11. Audit log and recovery after Juggler itself crashes

### 11.1 Audit log

Append-only `cleanup_audit` (SQLite table + JSONL export): timestamp,
actor (`user:<command>` | `auto:tier-1` | `auto:tier-2`), candidate
PID/PGID/exe/cmdline-hash, Instance/Provider/Account/Project,
identity-check results, scope chosen, signals sent with per-PID
outcomes, ports before/after. Written *before* the first signal
(intent) and completed after (outcome) — a crash mid-ladder still
leaves a truthful “we TERM’d PGID 8120, outcome unknown” record.
The Cleanup Centre History filter (§9.1) reads this log; Settings
offers export. No audit record, no kill: if the audit write fails,
manual kills warn-and-require-retry and automatic tiers stand down.

### 11.2 Juggler crash recovery

Today: children survive drop by design
(`crates/juggler-supervisor/src/supervisor.rs:47-51`), the journal
replays roots (`crates/juggler-supervisor/src/journal.rs:151-196`),
and `AppState` reconciles non-adopted `Starting|Running` sessions to
`Stopped` (`crates/juggler-app/src/state.rs:53-72`). Gaps pass 1
named (F-1/F-3) plus two it did not: helpers are absent from the
journal so a crashed Juggler forgets every dev server it ever saw,
and adopted roots get no `ChildHandle` so they cannot be stopped.

Recovery sequence with the ledger: (1) load ledger rows in
`Running|Orphaned|Adopted`; (2) identity-verify each (§6.2) —
failures become `Unverifiable` (listed, never killed); (3) rebuild
adopted handles (output unavailable, stop = verified PGID ladder) —
closing F-3; (4) helpers of roots that died *while Juggler was down*
enter `Orphaned` with their original first-seen attribution, so the
user sees “left behind while Juggler was closed”, distinct from
live-session orphans; (5) show a one-time recovery summary (“3
Instances re-adopted, 2 helpers left behind, 1 process could not be
verified and was left alone”) — silence here is how trust dies.
Auto-cleanup tiers stay suspended (§10) until dismissed.

## 12. Supervisor architecture changes (Rust-level)

New module `crates/juggler-supervisor/src/tree.rs` (ledger types,
reconciler) and `ports.rs` (`PortProbe`); `lib.rs` exports grow from
today’s five modules (`crates/juggler-supervisor/src/lib.rs:14-24`).
Core types (signatures are the contract; bodies follow §5–§11):

```rust
pub struct ProcessIdentity { pub pid: u32, pub pgid: u32,
    pub start_time: i64, pub exe: PathBuf, pub boot_id: String }
pub enum HelperRole { Harness, Subagent, DevServer, McpServer,
    TerminalShell, Watcher, BuildTool, Unknown }
pub enum OrphanState { Running, Exited, Orphaned, Adopted, Unverifiable }
pub struct LedgerRow { pub identity: ProcessIdentity,
    pub session: SessionId, pub root_pid: u32, pub parent_pid: u32,
    pub role: HelperRole, pub state: OrphanState,
    pub ports: Vec<u16>, pub peak_rss: u64 }
pub trait LedgerStore: Send + Sync {
    fn intent(&self, s: &SessionId) -> Result<LedgerId, SupervisorError>;
    fn spawned(&self, id: LedgerId, p: &ProcessIdentity) -> Result<(), SupervisorError>;
    fn upsert_helpers(&self, rows: &[LedgerRow]) -> Result<(), SupervisorError>;
    fn candidates(&self) -> Result<Vec<LedgerRow>, SupervisorError>;
}
pub fn verify(pid: u32, expect: &ProcessIdentity) -> Verification; // §6.2, fail-closed
pub fn terminate_group(pgid: u32, grace: Duration) -> LadderOutcome; // TERM→poll→KILL
```

Spawn changes: piped path gains `pre_exec(setsid)` + PGID capture
(against today’s bare `Command` at
`crates/juggler-supervisor/src/supervisor.rs:129-140`); PTY path
records `child.process_id()`’s PGID explicitly
(`supervisor.rs:214-219`). `Supervisor::stop` keeps its signature for
the two existing callers but delegates to `terminate_group` over the
verified tree — F-9 fixed at the primitive, not by bolting a ppid
walk onto `stop`. `descendants()` is kept for compatibility but
reimplemented on the reconciler’s cached map. `ChildHandle` gains
`identity: ProcessIdentity` and adopted-handle construction.
Errors: add `SupervisorError::Unverifiable { pid }` and
`::LedgerDegraded` to the closed enum at
`crates/juggler-supervisor/src/error.rs:10-39` — callers must handle
“refused to kill” as a typed outcome, never as `UnknownSession`.
No new `unsafe` beyond the existing `nix` signal calls
(`crates/juggler-supervisor/Cargo.toml` unix `nix` dep); `pre_exec`
closure must be async-signal-safe (single `setsid` call only).

## 13. Safety FMEA (cleanup-specific; extends Maximum-Depth §5)

| # | Failure | Harm | Mitigation in this design |
|---|---|---|---|
| C1 | PID reused by user’s own process, then killed | Data loss in an unrelated app — the worst outcome this feature can produce | §6.2 triple check at kill-time; T5 rows have no kill affordance (§9.2) |
| C2 | Helper `setsid`-escapes PGID; group kill misses it, UI claims success | False confidence; port stays held | Post-ladder port re-probe (§9.3); escaped helpers labelled, killed PID-only after re-verify |
| C3 | Shared PGID with user’s terminal shell | Killing the group kills the user’s shell | Spawn-time dedicated PGID per Instance (§4); confirm sheet lists every PGID member (§9.2) |
| C4 | MCP server holds unsaved state; Tier-1 kills it as “watcher” | Work loss | Role exclusions (§10); role is a guess, so only `DevServer|Watcher` are ever auto-eligible |
| C5 | Adopted survivor (T3) mistaken for orphan after Juggler crash | Kills work the user deliberately kept (SPEC §5.3, `docs/SPEC.md:251`) | `Adopted` is a separate state; Tiers 1–2 exclude it; recovery summary (§11.2) |
| C6 | Fork storm floods ledger/UI | Perf death, alarm fatigue | 2-tick confirmation, batched upserts, collapsed healthy trees (§6.3, §9.1) |
| C7 | Audit/ledger write fails, kill proceeds unrecorded | Unaccountable destruction | §11.1: no record, no kill (auto) / warn-and-retry (manual) |
| C8 | Clock/boot change makes start-time checks flaky | Everything flips `Unverifiable`, feature looks broken | `boot_id` gate first; mass-unverifiable triggers a health banner, not row spam |

## 14. Acceptance criteria (testable, fixture-first)

Requires the SPEC §10 fixture harness (`docs/SPEC.md:439`) extended
to spawn a port-holding, TERM-ignoring, and `setsid`-escaping helper:

1. Fake Instance spawns vite-like helper on :5173 → ledger shows
   helper + port within 2 ticks; root exits cleanly → helper lists
   as Orphaned only after grace, with correct Provider/Account/Project.
2. `stop` on a live tree kills root *and* helpers (extends
   `crates/juggler-supervisor/tests/supervisor.rs:153-173`, which
   today never asserts helper death); :5173 verifiably free after.
3. PID-reuse drill: kill helper, respawn a stranger reusing the PID
   (or simulated identity mismatch) → row becomes `Unverifiable`,
   no kill control rendered, automatic tiers skip it.
4. Juggler-crash drill: drop supervisor with live tree (pattern of
   `tests/supervisor.rs:118-151`), restart → roots re-adopted *with*
   stop control, down-period helpers listed as left-behind, recovery
   summary shown once.
5. Tier-1 drill (opt-in): clean-exit orphan dev-server auto-TERMed
   after grace with pre-written audit record; MCP-role helper in the
   same tree untouched. Default install (Tier 0) signals nothing —
   asserted by a guardrail-style test alongside the invariant at
   `docs/SPEC.md:26`.
6. Budget: reconciler at 50 Instances keeps supervisor CPU < 1%
   (SPEC budget, `docs/SPEC.md:24`) and adds no per-query
   `System::new()` (regression test counts refreshes per tick).

Sequencing: ledger + PGID + reconciler first (unblocks F-2/F-3/F-9), Cleanup Centre read-only second, manual ladder third, Tier 1/2 last and only after the SPEC automation amendment. That is M2-6/M2-7 scope (`docs/plan/EPICS.md:80`) plus one new task, **M2-9 Cleanup Centre**, gated at V2.


# 06 — External Developer Pain Research (Second Pass)

> Parent: maintainer canonical workflow in
> [expected-workflow.md](../internal/expected-workflow.md#L1).
> Prior research cited as: Maximum-Depth Report (MDR) §01–§13 in
> [AI-Juggler-Maximum-Depth-Report-and-Proposal.md](AI-Juggler-Maximum-Depth-Report-and-Proposal.md#L1)
> and Competitive UX Review (CUX) F1–F7 in
> [full-competitive-ux-review.md](full-competitive-ux-review.md#L1).
> This file does not restate either report. It extends them where
> external developer evidence was under-weighted, wrong, or missed.
> Method: 20 web searches (log in §12), 2026-10-13. Every claim
> carries a repo `file:line`, a prior-report section, or an
> external URL. `[verified]` = direct external source read;
> `[inferred]` = synthesis, discount accordingly.

## 0. Second-pass verdict in eight lines

- Prior reports are strong on quota visibility and handover
  mechanics (MDR §06, §09; CUX F1.5, F3) but treat the quota
  wall as a *display* problem. External evidence shows it is a
  *timing and trust* problem: developers cannot predict resets,
  do not trust the percentage, and discover exhaustion only
  mid-task (see §2). `[verified]`
- Prior reports under-weight *silent* failures: wrong-account
  billing without any UI signal, orphan dev servers holding
  ports for days, and MCP/config fork per account. Each has
  shipped community workarounds Juggler does not yet spec
  against (see §4, §6, §8). `[verified]`
- Prior reports mark worktree collisions `[inferred]` (CUX
  pain #7). External evidence promotes this to verified,
  repeated data loss — silent overwrite with no conflict
  marker (see §7). `[verified]`
- The maintainer workflow demands switching "as many times as
  I want, back and forth" in
  [expected-workflow.md](../internal/expected-workflow.md#L7).
  Prior handover design (MDR §09) is A→B→A linear. External
  switcher tools already do pool round-robin across 3+ seats;
  Juggler's lineage model has no pool view (see §3). `[verified]`
- Gemini CLI is absent from SPEC adapters
  ([SPEC.md](../SPEC.md#L206))
  yet external pain (429 tier mismatch, per-family quotas) is
  structurally identical to Claude/Codex. Deferring it leaves
  the "multiple tools" promise in
  [expected-workflow.md](../internal/expected-workflow.md#L3)
  unproven for a third of the named workflow. `[inferred]`

## 1. Terminology: maintainer language vs. repo domain

Maintainer language is normative per
[expected-workflow.md](../internal/expected-workflow.md#L11):
"we should call them providers, accounts, and instances of
the tool". The repo does not yet speak this language.

| Maintainer term | Repo term today | Where | Conflict |
|---|---|---|---|
| Provider | Harness / `HarnessId` | [SPEC.md](../SPEC.md#L91) | Rename risk |
| Account | `Account` | [SPEC.md](../SPEC.md#L93) | Aligned |
| Instance (tool instance) | `SessionRecord` + Instance UI | [SPEC.md](../SPEC.md#L108) | Split model |
| — (no maintainer word) | `Profile` | [SPEC.md](../SPEC.md#L101) | Extra concept |

Findings:

- **F-T1. `Provider` does not exist in the domain model.**
  `HarnessId` (`"claude-code" | "codex" | "zcode"`) in
  [SPEC.md](../SPEC.md#L91)
  is the provider, but UI copy, onboarding, and the expected
  workflow all say "provider". New users meet two words for
  one thing. Recommendation: rename the user-facing label to
  Provider everywhere; keep `HarnessId` as the internal type
  only if a migration note records the alias. `[verified]`
- **F-T2. `Profile` is an unmaintainer concept users must
  learn.** `Profile = account + settings_overlay` in
  [SPEC.md](../SPEC.md#L101)
  has no counterpart in the maintainer workflow, which speaks
  only of switching accounts on a project
  ([expected-workflow.md](../internal/expected-workflow.md#L5)).
  CUX F2.2 already flags Instances ≠ Sessions conflation;
  Profile adds a third noun. Recommendation: demote Profile
  to an advanced/settings concept; default UX speaks
  Provider → Account → Instance only. `[inferred]`
- **F-T3. `Instance` is split across two repo concepts.**
  SPEC §9.1 defines Instances as "a CLI running in an
  isolated terminal *or* a desktop harness window" in
  [SPEC.md](../SPEC.md#L320),
  while `SessionRecord` in
  [SPEC.md](../SPEC.md#L108)
  is the persisted conversation. Users think "my instance"
  = one running tool; the repo forces them to also think
  "session inside instance". Recommendation: make Instance
  the primary noun (one running tool process tree); Session
  becomes its conversation history, shown inside the
  Instance panel, never as a peer top-level object.
  `[inferred]`

## 2. Pain cluster P1 — Quota walls hit mid-task, without warning

**What prior research said.** MDR §06 grades quota numbers by
honesty tier; CUX F1.5 asks for a reset-queue. Both frame the
fix as better display plus scheduled continuation.

**What external evidence adds.** The wall is discovered
*during* work, not before it, and the in-flight work is the
casualty — a dimension prior reports mention only via repo
issues (PROPOSAL §2) without external corroboration.

- "You've probably been blindsided mid-conversation when
  Claude just stops and you realize you hit the 5-hour
  limit" — menu-bar tool built solely to prevent this
  [aiquotabar POST_PLAN](https://github.com/yagcioglutoprak/aiquotabar/blob/HEAD/docs/planning/POST_PLAN.md).
  `[verified]`
- "When Claude Code runs out mid-task, you can carry on in
  Codex" is now a documented cross-tool handoff pattern, not
  an edge case [dev.to usage guide](https://dev.to/heylittlepan/how-to-check-your-claude-code-and-codex-usage-limits-34mf).
  `[verified]`
- Max 20x users report "jumps from 21% to 100% usage on a
  single prompt" and Max 5x windows exhausted in "~1.5 hours
  with normal agentic tasks"
  [claude-code #38335](https://github.com/anthropics/claude-code/issues/38335).
  `[verified]`
- A dedicated auto-retry tool exists whose entire job is to
  wait for the printed reset ("resets 3pm") and send
  "continue" automatically
  [claude-auto-retry](https://github.com/cheapestinference/claude-auto-retry).
  `[verified]`

| Signal | Reading |
|---|---|
| Frequency | Very high — 724 issues cite the 5h limit per [PROPOSAL.md](../PROPOSAL.md#L23); plus standalone retry/quota-bar tools |
| Severity | High — in-flight task aborts; context must be rebuilt elsewhere |
| Workaround today | Menu-bar quota widgets, auto-retry scripts, manual tool switch |
| Juggler opportunity | Pre-flight forecast + mid-task early warning, not just rings |

**Feature implication.** SPEC §6 burn-down projection
("exhausts in ~40 min") in
[SPEC.md](../SPEC.md#L257)
must fire *inside* the running Instance as a passive alert
at configurable headroom (e.g. 20% left), with a one-click
"prepare handover" staging action. A dashboard ring the user
must go and look at does not solve a mid-task wall.
`[inferred]`

## 3. Pain cluster P2 — Reset timing is opaque and distrusted

**What prior research said.** MDR §06 notes native baselines
rose (statusline `rate_limits`, Codex `/status`). It does not
interrogate whether users *believe* the numbers.

**What external evidence adds.** Three distinct trust
failures, none in prior reports:

- **Direction confusion.** "Claude Code and Grok show how
  much you've used. Codex shows how much is left. '30%' in
  Claude Code is close to '70% left' in Codex"
  [dev.to usage guide](https://dev.to/heylittlepan/how-to-check-your-claude-code-and-codex-usage-limits-34mf).
  A unified Juggler ring that does not normalise direction
  will mislead exactly the multi-tool users it targets.
  `[verified]`
- **Contradictory windows.** "When the weekly usage limit
  reaches 0%, the 5-hour limit can still show substantial
  remaining capacity (63%) even though that capacity is
  unusable. This is misleading UX"
  [OpenAI Community](https://community.openai.com/t/codex-usage-ui-is-misleading-when-weekly-reaches-0-but-5h-still-shows-remaining-quota/1404599).
  `[verified]`
- **Reset semantics unknown.** "Claude does not reset at
  midnight. Your limits operate on a dynamic rolling
  window" and "starting a new conversation doesn't reset
  either window" are facts users must learn from third-party
  guides, not from the tool
  [specweave guide](https://github.com/anton-abyzov/specweave/blob/HEAD/docs-site/docs/guides/claude-code-usage-limit.md).
  `[verified]`

| Signal | Reading |
|---|---|
| Frequency | High — every multi-tool user crosses both conventions daily |
| Severity | Medium — wrong "plenty left" reading causes the P1 wall |
| Workaround today | Third-party guides, statusline countdown scripts |
| Juggler opportunity | Single normalised "headroom + resets-at" language |

**Feature implication.** Every quota surface must show, in
one line: headroom remaining (never used-%), the binding
window (5h vs weekly, whichever exhausts first), and an
absolute reset time with countdown. SPEC §9.2 Overview
"quota rings with reset countdowns" in
[SPEC.md](../SPEC.md#L345)
should be amended to mandate the binding-window rule; a ring
per window invites the Codex 63%-but-unusable error.
`[inferred]`

## 4. Pain cluster P3 — Account switching is manual alias folklore

**What prior research said.** PROPOSAL §2 and MDR §06 cover
switchers (`cc-switch`, `cswap`, `aisw`) as competitors.
They under-state how *fragmented* the workaround ecosystem
is: every developer hand-rolls the same alias independently.

- "Switching is manual. You need to log out, log back in,
  or juggle `CLAUDE_CONFIG_DIR` yourself" and "settings
  don't carry over — MCP servers, plugins, permissions have
  to be set up from scratch, every time"
  [clausona](https://github.com/scalabl3/clausona).
  `[verified]`
- The canonical workaround is a shell alias pair:
  `alias claude-account1="CLAUDE_CONFIG_DIR=~/.claude-account1 claude"`
  [GitHub Gist](https://gist.github.com/sonnyit/20f7198c27be7ad92606203fdfc66019).
  `[verified]`
- Rotation tools pick "the account with most 5h/weekly
  quota headroom" automatically
  [claude-rotate](https://github.com/evrenverse/claude-rotate);
  another polls "each account's usage via the same OAuth
  endpoint Claude Code itself uses for `/usage`"
  [claude-usage-swap](https://github.com/jonazri/claude-usage-swap).
  `[verified]`
- HN validated the pain: "Show HN: Claude-account – switch
  Claude Code accounts without logging in again" (47 points,
  24 comments); typical reaction "why isn't this built in"
  [HN digest](https://github.com/xiehd77-del/agents-radar/issues/361).
  `[verified]`
- The feature request for native support asks for
  `claude auth list / switch / add` modelled on `gh auth`
  [claude-code #37554](https://github.com/anthropics/claude-code/issues/37554).
  `[verified]`

| Signal | Reading |
|---|---|
| Frequency | Very high — 6+ independent switcher tools plus gist folklore |
| Severity | Medium — daily friction; error-prone under time pressure |
| Workaround today | Aliases, snapshot-swap scripts, auto-rotators |
| Juggler opportunity | Managed Accounts with zero shell knowledge required |

**Where prior research is wrong.** MDR §06 treats
auto-rotation as a competitor behaviour to avoid (correct
per PROPOSAL guardrail 2 in
[PROPOSAL.md](../PROPOSAL.md#L63)).
But it misses that rotation tools prove users want a *pool
view*: all Accounts of one Provider ranked by headroom, with
the switch one click away. Juggler's per-Account cards in
[SPEC.md](../SPEC.md#L346)
have no ranked-pool ordering. **Feature implication:** the
Accounts screen and the handover target list must sort by
live headroom descending by default, so the compliant
(user-clicked) switch feels as fast as the non-compliant
auto-rotator. `[inferred]`

## 5. Pain cluster P4 — `CLAUDE_CONFIG_DIR` isolation forks everything

**What prior research said.** SPEC §3 presents
`CLAUDE_CONFIG_DIR` isolation as the verified mechanism in
[SPEC.md](../SPEC.md#L132).
MDR §03 audits onboarding import. Neither report prices the
*ongoing* cost of isolation: everything forks per Account.

- "`CLAUDE_CONFIG_DIR` moves the entire data directory, so
  conversation history and memory fork per account"
  [sub-switcher README](https://github.com/whi02/sub-switcher/blob/HEAD/README.en.md).
  `[verified]`
- "Each account gets its own config directory, so your MCP
  servers, plugins, permissions, and settings have to be
  set up from scratch — every time"
  [clausona](https://github.com/scalabl3/clausona).
  `[verified]`
- Cross-tool MCP duplication is the same disease one level
  up: "Configure MCP once, use it in Claude, Cursor, and
  Codex" exists because "each client made its own choice"
  for config location and format
  [Medium, MCP gateway](https://medium.com/@jaegercode/configure-mcp-once-use-it-in-claude-cursor-and-codex-6685b5d28d08).
  `[verified]`
- Duplicate MCP processes launch silently when two config
  sources declare the same server
  [claude-code #66474](https://github.com/anthropics/claude-code/issues/66474).
  `[verified]`

| Signal | Reading |
|---|---|
| Frequency | High — hits every added Account, every added MCP server |
| Severity | Medium — hours of re-setup; silent duplicate processes |
| Workaround today | Hand-copied configs, `--mcp-config` overlays, symlinks |
| Juggler opportunity | Shared-settings layer across Accounts of a Provider |

**Feature implication.** CUX F1.9 ("settings/MCP/rules
portability") is scoped as a P2 checkbox sync. External
evidence says this is *onboarding-blocking*: the second
Account is unusable until MCP/plugins/permissions are
rebuilt. Promote to P1: on Account creation, offer "copy
shared settings from Account X" (MCP servers, plugins,
permissions, model defaults) as a default-on step in the
wizard, diff-previewed. Credentials never copy (SPEC §8 in
[SPEC.md](../SPEC.md#L310)).
`[inferred]`

## 6. Pain cluster P5 — Context loss on switch; rules duplication

**What prior research said.** MDR §09 designs handover
tiers T1–T4; SPEC §7.2 flow 4 syncs rules files in
[SPEC.md](../SPEC.md#L289).
Both assume the *conversation* is the context that matters.

**What external evidence adds.** Developers lose three
contexts on switch, and prior reports price only the first:

1. **Conversation context** (priced): "switching accounts
   means logging out, logging back in, and losing whatever
   session state you had"
   [dev.to multi-account guide](https://dev.to/sakibullah2006/running-multiple-claude-code-accounts-on-linux-opj).
   `[verified]`
2. **Rules context** (under-priced): teams maintain
   `CLAUDE.md`, `AGENTS.md`, `.cursorrules`,
   `copilot-instructions.md` in parallel; tooling now exists
   solely to keep them "single-sourced and honest" with
   `CLAUDE.md` as a one-line `@AGENTS.md` include
   [agent-standard](https://github.com/anmoln7/agent-standard-oss).
   ETH Zurich research cited there finds auto-generated
   context files "reduce agent success rates ~2% and
   increase costs 20%+"
   [quality-of-my-claude-md](https://github.com/riyazsarah/quality-of-my-claude-md).
   `[verified]`
3. **Working context** (unpriced): uncommitted diff, running
   dev server, shell env, and in-session todos do not travel
   with a transcript copy. SPEC §7.3 packs cover diffs in
   [SPEC.md](../SPEC.md#L303)
   but no prior section inventories dev-server/env state.
   `[inferred]`

Positive signal: "Claude Code v2.1.277 reads AGENTS.md when
CLAUDE.md is missing" removes one duplication, but "leaves
the other (keys, billing, model switching) untouched"
[dev.to](https://dev.to/cogumellum/claude-code-v21277-reads-agentsmd-when-claudemd-is-missing-53ln).
SPEC already notes this in
[SPEC.md](../SPEC.md#L289).
`[verified]`

| Signal | Reading |
|---|---|
| Frequency | High — every cross-Provider or cross-Account switch |
| Severity | High — re-explaining costs 10–30 min and loses decisions |
| Workaround today | Symlinks, `@AGENTS.md` imports, copy-paste briefs |
| Juggler opportunity | Handover carries rules + diff + env manifest, verified |

**Feature implication.** The handover wizard must show a
context checklist (conversation ✓, rules files ✓/diff,
uncommitted diff ✓, MCP/settings ✓, dev-server/env —
declared or explicitly dropped) before execute. Today SPEC
§7.2 in
[SPEC.md](../SPEC.md#L287)
lists fidelity tier + quota headroom only. A switch that
silently drops the dev server state recreates the pain
inside Juggler. `[inferred]`

## 7. Pain cluster P6 — Wrong-account billing is silent

**What prior research said.** CUX job #1 names wrong-billing
("Start the right account on the right repo without
wrong-billing"); PROPOSAL Persona B in
[PROPOSAL.md](../PROPOSAL.md#L52)
cites Claude #87447. Both treat it as a *selection* error
by the user.

**What external evidence adds.** The billing can be wrong
*without* any user selection error, and the UI actively
masks it — a failure class absent from prior reports:

- Headless `claude -p` spawned from an Account E session
  "authenticates as Account P — the machine-default macOS
  Keychain credential. The mismatch is silent, and
  `claude auth status` reports Account E, masking it. Usage,
  billing, and rate limits are charged to the wrong account"
  [claude-code #72552](https://github.com/anthropics/claude-code/issues/72552).
  `[verified]`
- "`/status` displays wrong account/email — session actually
  authenticated and billed against a different (Enterprise)
  org" while showing the personal account name
  [claude-code #74217](https://github.com/anthropics/claude-code/issues/74217).
  `[verified]`

| Signal | Reading |
|---|---|
| Frequency | Low–medium — needs nested/headless spawn to trigger |
| Severity | Critical — money and quota charged to the wrong Account |
| Workaround today | None; discovery is post-hoc via billing pages |
| Juggler opportunity | Per-Instance identity assertion, continuously shown |

**Feature implication.** This is the strongest argument for
the maintainer's "icons and labels — which provider,
account, models currently running" demand in
[expected-workflow.md](../internal/expected-workflow.md#L13).
Every Instance header and every spawned child must carry a
verified Account badge derived from the *effective*
credential (keychain namespace / `CODEX_HOME`), not from
the launch intent. Add a supervisor check: child process
env `CLAUDE_CONFIG_DIR`/`CODEX_HOME` must equal the
Instance's managed dir; mismatch raises a visible
wrong-account alert before first model call. `[inferred]`

## 8. Pain cluster P7 — Zombie/orphan processes and port conflicts

**What prior research said.** The maintainer workflow names
this explicitly: subprocesses "end up being zombie
processes" needing "automatic and manual clean ups, all
with verifications" in
[expected-workflow.md](../internal/expected-workflow.md#L17).
SPEC §5.1 covers re-adoption on restart in
[SPEC.md](../SPEC.md#L224).
Prior reports do not evidence the *port* consequence.

**What external evidence adds.** The orphan's damage is a
held port discovered days later, plus parallel-agent port
fights — both with shipped community fixes Juggler should
meet or beat:

- "The model starts a dev server with `(cmd &)`, the session
  ends, and the server keeps holding its port. Days later
  you get `address already in use`, and nothing else tells
  you the server still runs" — a plugin now lists such
  servers "with their age and session and a stop button"
  [orphan-server](https://github.com/kilimcininkoroglu/claude-code-mods/blob/HEAD/plugins/orphan-server/README.md).
  `[verified]`
- Parallel worktree agents "fought over the same port";
  fix was per-worktree ports 5181–5190 plus `--strictPort`
  so "a taken port fail[s] loudly"
  [Artisan Defense field report](https://helgesver.re/articles/how-i-built-artisan-defense).
  `[verified]`
- "Two dev servers are fighting over port 3000" is a
  canonical parallel-agent failure
  [parallel agents guide](https://medium.com/ai-systems-lab/how-to-run-claude-code-agents-in-parallel-setup-guide-1d882947509e).
  `[verified]`
- A PreToolUse hook now "detects port conflicts before
  server startup… blocks the action and suggests an
  alternative" when the port belongs to another project
  [port-saver](https://github.com/gtpgg1013/port-saver).
  `[verified]`
- Orphaned plugin-cache leases from dead PIDs "pinned
  permanently" because "nothing reaps a lease whose owning
  process has exited"
  [claude-code #82145](https://github.com/anthropics/claude-code/issues/82145).
  `[verified]`

| Signal | Reading |
|---|---|
| Frequency | High — any Instance that starts a server, daily for parallel users |
| Severity | Medium–high — blocked launches, stale servers serving old code |
| Workaround today | `lsof` archaeology, orphan-server/port-saver plugins |
| Juggler opportunity | Process-tree ownership + port registry per Instance |

**Feature implication.** Juggler's supervisor already tracks
process trees (SPEC §5.2 in
[SPEC.md](../SPEC.md#L234)).
Extend it: (a) record listening ports per Instance tree;
(b) on Instance stop, list surviving children with age and
port and offer verified cleanup (confirm PID + port freed);
(c) on launch, pre-flight the project's known dev ports and
warn on collision with another Instance. This directly
implements the maintainer's verified-cleanup demand.
`[inferred]`

## 9. Pain cluster P8 — Parallel Instances overwrite each other's work

**What prior research said.** CUX F1.1 (P0) requires
worktree isolation; the persona pain table marks worktree
collisions `† [inferred]… Needs dogfood validation`.

**What external evidence adds.** Validation now exists, and
it is worse than inferred: the failure is silent.

- "Two agents in one checkout overwrite each other's edits
  with no error and no conflict marker — git sees a single
  working copy"
  [ai-skills worktrees](https://github.com/tschallacka/ai-skills/blob/HEAD/git-worktrees/SKILL.md).
  `[verified]`
- "Do NOT let multiple agents share the main checkout —
  they will silently overwrite each other's staged changes.
  Hit this 3 times during the 2026-05-06 perf sprint"
  [field note](https://github.com/murror/murrorastro/blob/HEAD/.claude/memory/feedback_isolated_worktree_for_parallel_agents.md).
  `[verified]`
- "Multiple agents staging/committing in the same worktree
  overwrite each other" plus `index.lock` races on a shared
  `.git`
  [datus CLAUDE.md](https://github.com/datus-ai/datus-db-adapters/blob/HEAD/CLAUDE.md).
  `[verified]`

| Signal | Reading |
|---|---|
| Frequency | High for the core persona (2–3 parallel Instances per [PROPOSAL.md](../PROPOSAL.md#L51)) |
| Severity | Critical — silent loss of agent work |
| Workaround today | Manual `git worktree add`, one branch per agent |
| Juggler opportunity | Worktree-on-launch as the parallel default |

**Correction to prior research.** Promote CUX pain #7 from
`[inferred]` to `[verified]`, and F1.1's acceptance must
include the dirty-tree case: launching a second Instance on
a project with uncommitted changes offers worktree / stash /
share-with-warning — never silent sharing. `[inferred]`

## 10. Pain clusters P9–P10 — Stuck agents, sprawl, and cost surprises

**P9: stuck/waiting Instances and session sprawl.** Prior:
CUX F2.5/F5.2 (statuses modelled, never emitted); MDR §02.
External: "The agent lives in a terminal… It stops to ask a
question while you're in a meeting, and the question sits
there for two hours"
[dev.to GetPullRequest](https://dev.to/getpullrequest-hq/your-ai-coding-agent-is-stuck-in-a-terminal-you-have-to-babysit-so-we-built-getpullrequest-1jng);
a queue tool now sorts sessions by "how long it has been
waiting… Permission prompts and questions first"
[dev.to session queue](http://dev.to/bargan/which-claude-code-session-is-waiting-on-you-a-queue-for-agents-that-stopped-to-ask-1ok9);
frozen sessions are "a stalled network connection… a tool
call stuck waiting on input… or a startup hang caused by a
slow-loading MCP server"
[Medium freeze guide](https://medium.com/@cartseoservice/claude-code-keeps-freezing-or-hanging-how-to-fix-it-5a6e07b79a85).
Frequency: daily at 5–15 Instances. Severity: high (idle
quota burn + blocked human). Workaround: terminal-tab
archaeology. **Implication:** the Needs-Attention inbox
(CUX WOW-a) must sort by waiting-time descending and show
*what* the Instance waits on (permission / question /
network), reusing SPEC stuck heuristics in
[SPEC.md](../SPEC.md#L220).
`[verified]` evidence, `[inferred]` implication.

**P10: cost surprises from invisible token burn.** Prior:
CUX F1.2 (no USD view); CUX pain #5. External: "A default
Claude Code subagent sends 10,545 tokens before it does any
work… Three subagents cost exactly three times one"
[Plain English measurement](https://ai.plainenglish.io/every-claude-code-subagent-pays-10-545-tokens-before-working-e29d414d3a18);
"a coding agent stuck in a retry loop… can spend about
$115 an hour… a warning email arrives after that money is
gone"
[dev.to spend limits](http://dev.to/akaranjkar08/claude-code-spend-limit-hard-caps-that-stop-a-runaway-bill-1ihi);
subscription users "burn within 2 hour of coding in
interactive mode and 20 minutes if we use multiple workers"
[HN #48505268](https://news.ycombinator.com/item?id=48505268).
Frequency: high for subagent-heavy users. Severity: high.
Workaround: ccusage post-hoc reports, hard-cap scripts.
**Implication:** per-Instance live burn rate (tokens/min,
est. USD where API-billed, %/min of window where
subscription-billed) in the Instance panel header — the
right-inspector demand in
[expected-workflow.md](../internal/expected-workflow.md#L14)
— plus a passive runaway alert at 3× the Instance's median
rate (extends SPEC §5.2 anomaly rule in
[SPEC.md](../SPEC.md#L243)
from RSS to burn). `[inferred]`

**Gemini note (missed Provider).** Paid-tier users hit
429s because the CLI "is often stuck in 'Free Tier' with
zeroed-out limits" until a separate perk is claimed
[gemini-cli #24396](https://github.com/google-gemini/gemini-cli/issues/24396);
free-tier users round-robin "multiple Google accounts…
locked out for 25 minutes"
[Medium round-robin](https://medium.com/@dasunnimantha777/how-to-get-almost-unlimited-free-gemini-access-round-robin-multiple-google-accounts-with-openclaw-d5f4afd275aa).
Same juggling shape, third Provider. SPEC adapters stop at
ZCode in
[SPEC.md](../SPEC.md#L206);
recommend a Gemini discovery spike before claiming the
multi-Provider workflow. `[verified]`

## 11. Synthesis: what changes in priority

1. **P6 wrong-account assertion → P0.** Silent mis-billing
   with a masking UI (Claude #72552, #74217) outranks chart
   polish; it is also the clearest proof for per-Instance
   Provider/Account/model badges. Extends CUX job #1.
2. **P8 worktree guard: inferred → verified P0.** Keep CUX
   F1.1; add the dirty-tree launch decision. No new scope.
3. **P4 shared-settings copy → P1** (from CUX F1.9 P2):
   second-Account usability blocker, not a nicety.
4. **P2 binding-window quota line → P0 display rule** for
   M3: headroom + binding window + absolute reset, always.
   Sharpens MDR §06 honesty tiers with a layout mandate.
5. **P7 port registry + verified cleanup → M2**, beside
   stuck detection (EPICS M2-5/M2-6): the maintainer's
   zombie demand with external proof of the port cost.
6. **No priority change:** handover tiers (MDR §09) and
   guardrails (PROPOSAL §5) stand; external switchers
   confirm the user-clicked pool view (§4) as the compliant
   way to match their speed. `[inferred]`

## 12. Search log (20 queries, 2026-10-13)

Quota/walls: `Claude Code rate limit quota exhausted mid-task Reddit`; `Codex CLI rate limit weekly quota hit wall
Hacker News`; `"rate limit" reset timing 5h window Claude Code reset confusion`. Switching:
`Claude Code CLAUDE_CONFIG_DIR multiple accounts switching
pain`; `multiple Claude subscriptions Max account switching
hack alias workaround GitHub`; `Hacker News Claude Code
multiple subscriptions juggling accounts discussion`;
`Codex CLI session resume context lost switching accounts
CODEX_HOME`; `wrong account billing Claude Code work
personal account charged GitHub issue`. Rules:
`AI coding agent context loss switching tools CLAUDE.md
AGENTS.md duplication`; `MCP config duplication per account Claude Code`. Processes:
`Claude Code zombie orphan processes dev server port
conflict left running`; `Claude Code stuck agent hanging session sprawl`; `parallel
AI agents git worktree branch conflict same repo
overwrite`. Cost/other Providers: `Claude Code cost
surprise token burn expensive subagents Reddit`; `Gemini CLI quota multiple accounts forum`.
Plus 5 follow-up page reads counted in the same pass. Unresolved: no live Reddit thread bodies were fetched
(search snippets only); Gemini ZCode-parity quota API
unverified; Codex Estimated-window accuracy untested
against a live account — flagged `[inferred]` where relied on.





# Pass 2 §07 — Derived Workflows & Scenarios Beyond the Example

> Parent: [Maximum-Depth Report](AI-Juggler-Maximum-Depth-Report-and-Proposal.md#L1) ·
> Canonical example: [expected-workflow.md](../internal/expected-workflow.md#L1) ·
> Prior personas: [07-personas-and-journeys.md](deep/07-personas-and-journeys.md#L1) ·
> Prior feature PRDs: [10-product-proposal-missing-features.md](deep/10-product-proposal-missing-features.md#L1)
> Task: second pass — what prior research under-weighted, got wrong,
> or missed versus the maintainer's example. This file does not restate
> the prior report; each workflow cites it, then extends it.

## 0. Method and reading rules

The example workflow is one happy path: start on one tool, hit a quota wall, switch tools/accounts "as
many times as needed," with history, rules, and context intact
([expected-workflow.md:5](../internal/expected-workflow.md#L5),
[expected-workflow.md:7](../internal/expected-workflow.md#L7)). Prior research optimised around that noon wall:
forecast (10 §1), inbox (10 §2), honest handover (UX.5, full review F3), reset-return (SPEC §7.2,
[SPEC.md:288](../SPEC.md#L288)). All eleven workflows below start where that model ends — at night, at
scale, across projects, across people, or when a whole Provider is down.

Marks in this file: `[repo]` = code/docs-read fact with `file:line`; `[prior §]` = cited prior-report
section; `[inferred]` = reasoned extension needing dogfood validation. Time-saved figures are
`[inferred]` estimates against the manual workaround named in each workflow, not measured values; §13
states how to falsify them.

Automation rule applied to every workflow, verbatim: the app *prepares*, the user *executes*; automation
is passive only — monitoring, indexing, notifications, suggestions, staging
([SPEC.md:26](../SPEC.md#L26)); "the app never auto-rotates accounts"
([PROPOSAL.md:62](../PROPOSAL.md#L62)); "preparation and staging can be automated; execution is always a
user click" ([PROPOSAL.md:63](../PROPOSAL.md#L63)). Every workflow therefore splits **Auto (passive)**
from **Click-confirmed**. Anything that would auto-launch, auto-resume on a different Account, auto-kill,
or auto-send is listed as click-confirmed or as an anti-feature.

## 1. Terminology — adopt Provider / Account / Instance

The maintainer names the stack explicitly: "we should call them providers, accounts, and instances of the
tool" ([expected-workflow.md:11](../internal/expected-workflow.md#L11)). The codebase does not yet speak that
language. Mapping and conflicts:

| Maintainer term | Repo term today | Repo cite | Conflict |
|---|---|---|---|
| Provider | Harness / harness adapter | SPEC §4, [SPEC.md:91](../SPEC.md#L91) | Rename in UI copy; keep `HarnessAdapter` in code |
| Account | Account (state dir + login) | [SPEC.md:93](../SPEC.md#L93) | None — already correct |
| Instance (tool instance) | Instance (ADR-0007) vs Session | [adr/0007:46](../adr/0007-instance-centric-information-architecture.md#L46) | UI conflates the two (F2.2) |
| Profile | Profile = Account + settings overlay | [SPEC.md:101](../SPEC.md#L101) | No maintainer word; hide behind Instance launch |
| Session | SessionRecord (conversation) | [SPEC.md:108](../SPEC.md#L108) | Keep as the conversation inside an Instance |

Three naming conflicts to rule on before any workflow ships:

1. **Provider vs Harness.** SPEC and all adapters say "harness"; the maintainer and the user-facing copy
say "Provider" ([expected-workflow.md:13](../internal/expected-workflow.md#L13): "which provider it is").
Recommendation: UI and docs say Provider; `HarnessAdapter` stays as the code-level trait name. Do not
rename crates for a copy change. 2. **Instance vs Session.** ADR-0007 already defines Instance as "one
running/tracked occurrence of a harness under an account" and Session as "the work conversation living
inside an instance" (adr/0007 decision 1). The built UI renders Instances as filtered Sessions (full
review F2.2, [review:171](full-competitive-ux-review.md#L171)). Every workflow below uses
Instance = the running tool process (with CPU/RAM/children) and Session = the resumable conversation. 3.
**Profile has no user word.** A Profile is launch configuration, not a thing users "juggle." Workflows
below say "launch an Instance on Account B" and treat Profile as an advanced detail inside the launch
sheet, never as a rail noun.

The example also demands per-Session indicators — Provider, Account, model, quota left
([expected-workflow.md:13](../internal/expected-workflow.md#L13)) — and, on selection, a right-hand panel with
usage, history, quality, performance, CPU/RAM, and spawned sub-processes
([expected-workflow.md:14](../internal/expected-workflow.md#L14),
[expected-workflow.md:15](../internal/expected-workflow.md#L15)). Workflows assume that panel exists; §12 lists
where prior work under-built it.

## 2. What the prior pass under-weighted, got wrong, or missed

Cited to the prior reports, extended here — not restated:

1. **Time-of-day blindness.** Prior journeys are daytime and single-sitting (07 §9 journey maps; 10 §28
three flows at 08:00, 13:00, Friday-afternoon). Overnight, offline, and end-of-week cadences have no
journey. → W1, W8, W10 below. 2. **The wall is treated as an event, not a schedule.** 10 §9 (reset queue)
is the strongest prior answer, but it queues one Session. Nobody modelled *parking many* Sessions and
triaging them at reset. → W2, W3. 3. **Handover is modelled as same-task, same-person, same-day.**
Pair-review (W5), cheap→strong model arbitrage (W6), and cross-person handoff (W9) reuse the T1–T4 tiers
([SPEC.md:269](../SPEC.md#L269)) for different jobs the prior report never named. Prior handover trust
work (07 §7.2) covers fidelity labelling, not role-switching. 4. **One Project at a time.** 10 §15 groups
projects, but the consultant *day* — five clients, context-switch cost, billing boundaries — is a
workflow, not a grouping feature. → W4. 5. **Failure is per-Session; outage is per-Provider.** Crash
recovery is Session-scoped (SPEC §5.3, [SPEC.md:250](../SPEC.md#L250)); zombie/orphan cleanup gets one
line ([SPEC.md:224](../SPEC.md#L224)) against a full maintainer paragraph
([expected-workflow.md:17](../internal/expected-workflow.md#L17),
[expected-workflow.md:18](../internal/expected-workflow.md#L18)). A whole lab being down (W11) and incident
forensics (W7) are missing. 6. **"Team" was correctly scoped out, then over-applied.** Non-goal: "sharing
profiles or pools between people" ([PROPOSAL.md:45](../PROPOSAL.md#L45)). Prior work lets that veto even a
one-way, user-exported pack to a colleague. Export/import packs already exist (SPEC §7.2 flow 5,
[SPEC.md:290](../SPEC.md#L290)) — W9 keeps the guardrail and still ships the handoff.

## 3. Workflow catalogue at a glance

| # | Workflow | Persona anchor | Prior home | Status |
|---|---|---|---|---|
| W1 | Overnight / unattended runs | Devon (07 §2) | 10 §7 digest only | New |
| W2 | Quota-reset queue: park & resume | Maya (07 §1) | 10 §9 (single) | Extended |
| W3 | Morning triage of 15 Sessions | Devon (07 §2) | 10 §2 + §28.2 | Extended |
| W4 | Multi-project consultant day | Priya (07 §3) | 10 §15/§6 | Extended |
| W5 | Pair-switching for review | Devon + Maya | 10 §11 (gated) | New framing |
| W6 | Model arbitrage: cheap drafts, strong finishes | Alex (07 §6) | 10 §3 (account only) | New |
| W7 | Incident / debug workflow | Devon, Jonas (07 §4) | SPEC §5.3 partial | New |
| W8 | Offline / travel | Priya, Alex | `[NEG-EV]` none found | New |
| W9 | Team handoff: export pack to colleague | Priya, Sara (07 §5) | SPEC §7.2, gated by non-goal | Reframed |
| W10 | End-of-week cost review | Maya, Priya, Jonas | 10 §6 (feature, not ritual) | Extended |
| W11 | Provider outage day: failover whole Project | All | `[NEG-EV]` none found | New |

Each workflow follows the same shape: Trigger → Steps in Juggler (exact clicks, in maintainer language) →
Auto vs Click-confirmed → Failure branch → Delight moment → Features required (proposal IDs) → Journey
emotion + time saved `[inferred]`.

## W1. Overnight / unattended runs `[inferred]`

**Trigger.** 22:00. Devon (07 §2: 10–20 Sessions/day) leaves four Instances running refactors and tests;
Maya leaves one long migration. Today those Sessions either finish unseen, stall on a permission prompt
at 01:14, or die with the laptop lid (07 diary: "waited seven hours," 07 §2).

**Steps in Juggler.** (1) Instance panel → [Run overnight…] opens a pre-flight sheet:
Provider/Account/model chip, quota forecast pill ("A exhausts ~01:40 — park at wall or hand to B?"),
admission verdict (SPEC §5.2, [SPEC.md:237](../SPEC.md#L237)), battery/sleep warning. (2) User ticks
"Notify only if waiting > 15 min" and clicks [Start overnight]. (3) Overnight, the board keeps running;
the tray shows "4 overnight · 1 needs you." (4) 07:30, user clicks the Inbox row "Overnight digest (4)" →
per-Session outcome cards: Finished / WaitingUser / QuotaBlocked / Crashed, each with last-output excerpt
and token burn. (5) [Resume] / [Hand over…] per card.

**Auto (passive).** Status emission and stuck heuristics (SPEC §5.1, [SPEC.md:220](../SPEC.md#L220));
quota accounting and forecast; digest assembly; crash classification ([SPEC.md:250](../SPEC.md#L250));
re-adoption after app restart ([SPEC.md:251](../SPEC.md#L251)). **Click-confirmed.** Starting the
overnight run; any handover at the wall; any resume after a crash; dismissing a WaitingUser prompt (never
auto-answer permissions — anti-feature 7 in 10 §22).

**Failure branch.** Laptop sleeps or reboots: on wake, Sessions that died are surfaced with the same
recovery options (SPEC §5.3, [SPEC.md:252](../SPEC.md#L252)); the digest says "machine slept 02:10–06:40;
2 Sessions lost, both resumable" instead of pretending they ran. If quota exhausts at 01:40 with no
click, the Session parks as QuotaBlocked and enters W2's queue — it never auto-switches Accounts
([PROPOSAL.md:62](../PROPOSAL.md#L62)).

**Delight moment.** The 01:14 waiter is in the digest with "asked at 01:14, waited 6h 12m" — the exact
pain in Devon's diary becomes a number the tool owns. **Features required.** 10 §2 inbox, §7
notifications (digest mode), §1 forecast, §9 reset queue; SPEC §5.1 status emission is the hard
dependency (currently modelled, never emitted — full review F2.5). **Emotion + time saved.** Anxiety →
calm handover to the morning. `[inferred]` 20–40 min/day of tab-checking and re-prompting avoided;
falsify with 07 §11 diaries.

## W2. Quota-reset queue — park & auto-suggest resume `[inferred]`

**Trigger.** Maya hits the wall at 13:00 (07 §1; 10 §28.1). Two Sessions are mid-task; Account A resets
17:00; Account B has 20% left — too little to hand everything over.

**Steps in Juggler.** (1) Inbox row "QuotaBlocked · billing-migration · Account A" → [Park until reset].
(2) Park sheet: reset countdown (from the statusline `resets_at` feed, SPEC §4.1
[SPEC.md:192](../SPEC.md#L192)), target choice — "Return to A at reset" (default, SPEC §7.2 flow 3
[SPEC.md:288](../SPEC.md#L288)) or "Continue here on B." (3) Click [Park session]. Repeat for the second
Session; the Inbox gains a "Parked (2) · reset 17:00" group with a FIFO order the user can drag. (4) At
17:00, one notification: "Account A reset — 2 parked Sessions ready." (5) User clicks [Resume] on each,
in order; each resume is primed with the "while you were away" delta ([SPEC.md:288](../SPEC.md#L288)).

**Auto (passive).** Countdown timers, reset detection, the single reset notification, delta-pack staging,
queue ordering display. **Click-confirmed.** Parking, reordering if it changes which Session resumes
first on a shared window, and *every* resume — the queue "never launches unclicked" is already the
acceptance test in 10 §9. Extends 10 §9 from one Session to an ordered, multi-Session queue with a
shared-window warning ("resuming both at once re-exhausts A in ~50 min — stagger? [Stagger 20 min]
[Resume both]").

**Failure branch.** Reset time is Estimated (Codex, SPEC §4.2 [SPEC.md:203](../SPEC.md#L203)) and slips:
the queue shows "reset estimate stale — last checked 22m" and holds the Sessions parked rather than
resuming into a still-blocked Account. If the lineage broke (transcript moved), the row flips to `stale`
with an explanation (10 §9 controls), never a silent drop.

**Delight moment.** At 17:00 the user does two clicks, not a re-explanation of two tasks. **Features
required.** 10 §9 (extended to multi-Session), §1 forecast, §2 inbox grouping, §7 single interrupt at
reset. **Emotion + time saved.** Panic at the wall → "it is parked, it will ping me once." `[inferred]`
saves the 4-hour babysitting loop and ~15 min of re-prompting per parked Session.

## W3. Morning triage of 15 Sessions `[inferred]`

**Trigger.** 08:00. Devon opens the laptop to 15 Sessions from yesterday and overnight: 3 finished, 2
waiting on him, 1 stuck, 1 quota-blocked, the rest still running or stale (07 §2 day-in-life; rail caps
at 15/8 with no search today — full review F2.5).

**Steps in Juggler.** (1) App opens on the Inbox (10 §2: rail first item `Inbox (n)`), sorted by
waiting-time, oldest first. (2) `j`/`k` walks rows; each row shows Project · Account · Provider · model ·
why-chip · waiting timer · per-Session burn and footprint. (3) Oldest waiter: `Enter` → Instance panel
with the permission prompt focused → user answers in the terminal → row clears, badge decrements. (4)
Stuck? row: [View output] shows CPU/silence/mtime evidence (SPEC §5.1 [SPEC.md:220](../SPEC.md#L220)) →
[Resume] or [Hand over…]. (5) Finished rows: `x x x` selects three → [Approve → archive] (writes audit
rows, 10 §17). (6) `⌘K` → "stripe" → jumps to the one Session he actually needs.

**Auto (passive).** Waiting-time ordering, status classification, burn/footprint rollups, badge counts.
**Click-confirmed.** Every answer, resume, handover, and archive; snooze/mute per Session. Nothing is
auto-archived (retention archives only, never deletes — 10 §5, [SPEC.md:233](../SPEC.md#L233) growth
noted).

**Failure branch.** A status is a false Stuck? (model thinking, not hung): the evidence pane is the
escape hatch — the user sees "CPU active, transcript fresh 40s ago" and the chip downgrades itself. If
the watcher paused, the Inbox shows "Watcher paused — Resume" (10 §32) instead of an empty, lying list.

**Delight moment.** Seven blockers cleared keyboard-only in under four minutes (10 §28.2 target).
**Features required.** 10 §2 inbox, §10 palette, §18 keyboard, §5 archive; §11 review rows feed the same
queue. Prior 10 §28.2 scripts this morning but assumes the statuses exist; the extension here is
bulk-select + archive and the false-Stuck evidence pane. **Emotion + time saved.** Dread at 15 tabs →
tower control. `[inferred]` 25–45 min/morning versus clicking through terminals; measure with the §28.2
script.

## W4. Multi-project consultant day `[inferred]`

**Trigger.** Priya (07 §3): four clients, standups at 08:45, a screen-share at 11:00, an invoice Friday.
The risks are not quota — they are wrong-Account billing (Claude #87447,
[PROPOSAL.md:22](../PROPOSAL.md#L22) via research) and another client's name on screen.

**Steps in Juggler.** (1) Morning: Usage view filtered by client Group chip "Client B" (10 §15) → checks
yesterday's burn before standup. (2) Launch: NewSession on Client B's Project — the sheet shows
Provider/Account/model and warns "usual Account for this Project is Work Max; you picked Personal Max" →
[Use usual] / [Launch anyway] (10 §8 affinity). (3) 11:00 call: `⌘⇧P` → Present mode "Client B" — rail,
Inbox, and notifications show only Client B; a persistent banner reads "Presenting: Client B." (4)
Mid-day handover inside Client B: Session panel → [Hand over…] → wizard step 2 is the redaction preview
(07 §3: P0 for Priya, full review F1.6) → [Confirm handover]. (5) 17:30: [Export per-client pack] → CSV +
audit JSON for the invoice.

**Auto (passive).** Affinity computation (plurality of last 20 launches), per-client quota attribution,
Present-mode filtering (pure UI filter, 10 §15). **Click-confirmed.** Overriding the affinity warning,
every handover, every export; Present mode is a click and never auto-engages on a calendar event.

**Failure branch.** The affinity warning is wrong (a deliberate one-off): one click confirms and the
choice is remembered per Project+Account pair, so it does not nag twice. If a pack contains a likely
secret, the preview blocks [Confirm] until the user redacts or explicitly overrides — the override is
audit-logged (10 §17), because for Priya the log is the product.

**Delight moment.** The client asks "what did the AI do Tuesday?" and Priya exports the answer instead of
reconstructing it. **Features required.** 10 §15 grouping + Present mode, §8 account health/affinity, §6
cost ledger, §17 audit log, §14 MCP/settings sync for cloning a client setup. Extends 10 §28.3 (Friday
invoice) into the full day. **Emotion + time saved.** Low-grade dread → audit-ready calm. `[inferred]`
~45 min/week of spreadsheet allocation (07 §3) plus unquantifiable leakage-risk removal.

## W5. Pair-switching for review — author on A, review on B

**Trigger.** Maya authors a feature on Account A (her fast model); she wants a *different* Account —
ideally a different Provider — to review it, because same-model self-review misses its own mistakes
`[inferred]`. Prior 10 §11 gates a review queue on dogfood; this workflow is narrower: it never asks
Juggler to review, only to switch chairs.

**Steps in Juggler.** (1) Author Session finishes → Inbox row "Ready for review." (2) Click [Review on
another Account…] → target sheet ranked by Provider diversity first ("different Provider: fresh eyes"),
then headroom (10 §3 score, re-weighted). (3) User picks Account B / Provider 2 → wizard shows fidelity
tier (SPEC §7.1 [SPEC.md:269](../SPEC.md#L269): T2 same-Provider, T3 cross-Provider replay) and the
carry-list in plain words. (4) [Start review Session] launches an Instance on B primed with the full
conversation plus the diff; the author Session stays resumable, untouched (T2 copies, never moves —
[SPEC.md:287](../SPEC.md#L287)). (5) Reviewer Session leaves comments in its own terminal; user clicks
[Send comments to author Session] → staged as a primed resume on A, executed on click.

**Auto (passive).** Target ranking, diff computation (`git diff <base>...HEAD`, 10 §11), pack staging,
lineage linking. **Click-confirmed.** Starting the review Instance, sending comments back, any follow-up
fix launch. Juggler never auto-comments and never auto-merges — the review opinion belongs to the user.

**Failure branch.** No second Provider configured: the sheet says so and offers same-Account review with
an honest "same model family" label instead of faking independence. If the diff is empty, the row says
"no changes" (10 §11 controls) rather than launching a review of nothing.

**Delight moment.** "Authored by A, reviewed by B" appears as a lineage pair in the Project timeline —
the two chairs are visible. **Features required.** 10 §11 (read-only diff v1), §3 routing (re-weighted
for diversity), §17 lineage/audit; SPEC T2/T3 tiers. **Emotion + time saved.** Self-review unease → a
real second pair of eyes. `[inferred]` 15–30 min versus manual copy-the-diff into a second terminal, per
review; demand is `[inferred]` — keep behind the 10 §11 dogfood gate.

## W6. Model arbitrage — cheap model drafts, strong model finishes

**Trigger.** Alex (07 §6) and Jonas (07 §4) run small tanks: GLM Lite credits and Claude Pro, not Max.
Scaffolding, boilerplate, and test-writing do not need the strongest model; the final fix does. Prior 10
§3 ranks *Accounts* by headroom; it never ranks *models within* a task. `[inferred]` — no prior section
covers this split.

**Steps in Juggler.** (1) NewSession with template "Cheap grunt pass" (10 §16) → Provider GLM / budget
model on Account Lite → [Launch]. The Instance panel header shows Provider · Account · model · burn rate,
per the maintainer's indicator demand ([expected-workflow.md:13](../internal/expected-workflow.md#L13)). (2)
Draft lands; Inbox row "Draft ready." (3) Click [Finish with strong model…] → sheet: target
Provider/Account/model, fidelity tier (T3 cross-Provider replay [SPEC.md:273](../SPEC.md#L273) or T2), and
the estimated burn of *both* legs shown side by side. (4) [Hand over to strong model] → new Instance
primed with the full draft conversation; user deletes the scaffolding it got wrong. (5) Usage view shows
the split: "draft 82% on Lite ≈ cheap; finish 18% on Max" in sessions-remaining terms (07 §6: "≈N
sessions left," not raw credits).

**Auto (passive).** Burn attribution per leg, headroom ranking, "≈sessions left" translation, cost
estimate labelled Estimated (10 §6 label discipline). **Click-confirmed.** The handover itself and the
choice of strong target; Juggler may *suggest* the cheap leg at launch ("this looks like scaffolding —
start cheap?") but never downgrades a running Session's model.

**Failure branch.** The cheap draft is bad enough that replay wastes more strong-model tokens than it
saved: the sheet warns when the draft conversation exceeds a size threshold ("long draft — replay cost ≈
drafting fresh") and offers [Start fresh on strong model] as an equal first-class choice, not a hidden
fallback.

**Delight moment.** The Usage split makes the saving legible: "this feature cost one-sixth of an all-Max
run." **Features required.** 10 §16 templates, §3 routing (model-aware), §1 forecast, §6 cost ledger with
per-leg split; SPEC T2/T3. **Emotion + time saved.** Small-tank guilt → deliberate spending. `[inferred]`
the saving is quota/cost, not minutes: ~40–70% of a feature's burn moved to the cheap tier on
scaffolding-heavy work; validate by pricing two dogfood features both ways (10 §26 kill-tests).

## W7. Incident / debug workflow `[inferred]`

**Trigger.** Production is broken; or, locally, an Instance crashed and left children behind. The
maintainer's zombie paragraph is the seed: tools "crash, exit, and their subprocesses… end up running
long after the session closed… zombie processes" with "automatic and manual clean ups, all with
verifications" ([expected-workflow.md:17](../internal/expected-workflow.md#L17),
[expected-workflow.md:18](../internal/expected-workflow.md#L18)). Prior SPEC gives this one line
([SPEC.md:224](../SPEC.md#L224)) plus crash classification ([SPEC.md:250](../SPEC.md#L250)) —
under-weighted.

**Steps in Juggler.** (1) Tray/Inbox: "Instance crashed · api-server · 3 child processes still running."
Click → Instance panel incident card: exit signal, classification (OOM / quota / crash), process tree
with per-child CPU/RAM, and the transcript tail. (2) [Resume session] (native resume, transcripts
persist) or [Hand over…] if the Account is also quota-blocked. (3) Orphan sweep: Settings → Machine →
"Orphaned processes (3)" → table of PID, parent Session, age, command → user ticks rows → [Stop selected]
→ confirm sheet lists exactly what will be signalled (SIGTERM→SIGKILL ladder, SPEC §5.1
[SPEC.md:225](../SPEC.md#L225)). (4) Debug: [Export incident pack] → T4 pack (SPEC §7.3
[SPEC.md:297](../SPEC.md#L297)) plus machine snapshot, redacted by default ([SPEC.md:314](../SPEC.md#L314))
→ save/share manually.

**Auto (passive).** Crash detection/classification, process-tree attribution (ppid map, SPEC §5.2
[SPEC.md:234](../SPEC.md#L234)), orphan detection on restart, incident-pack assembly. Crucially, detection
is automatic; *killing is not* — see below. **Click-confirmed.** Every stop (per-row ticks + confirm),
every resume/handover, every export. Auto-kill of orphans would violate "no killing other applications"
([PROPOSAL.md:47](../PROPOSAL.md#L47): actions "on its own sessions only") and the passive invariant. The
maintainer asks for "automatic and manual clean ups… with verifications" — Juggler's compliant reading is
automatic *detection and staging*, click-confirmed cleanup.

**Failure branch.** A child PID was reused by an unrelated process (max-depth report P0: "journal
compares PID only," [report:162](AI-Juggler-Maximum-Depth-Report-and-Proposal.md#L162)): the sweep
must verify command line + start time before offering a row, and grey out anything it cannot attribute.
If attribution is uncertain, the row is informational only — never stoppable.

**Delight moment.** After a crash, one card answers "what died, what is still running, what do I click" —
no Activity Monitor archaeology. **Features required.** SPEC §5.1/§5.2/§5.3 (emit and surface them), 10
§2 inbox crash rows, §12 backup (incident pack rides the export path), §17 audit (every stop logged).
**Emotion + time saved.** Incident panic → a checklist. `[inferred]` 10–25 min per crash versus manual
`ps`/kill hunting, plus the avoided harm of killing the wrong process.

## W8. Offline / travel `[inferred]`

**Trigger.** Priya flies to a client; Alex works from a lab with flaky network (07 §6 pain: "flaky
network stalls login flows"). Quotas, statusline feeds, and harness-update checks all assume connectivity
the user temporarily does not have. No prior section covers offline operation; SPEC's only network uses
are the update feed and user-initiated harness actions ([SPEC.md:23](../SPEC.md#L23)), so the core is
already local-first — the *degradation copy* is what is missing.

**Steps in Juggler.** (1) Before travel, user opens Usage → per-Account cards show last-measured quota
with timestamps ("measured 09:12, 2h ago"). (2) On the plane, launch works fully (local CLIs); banners
state "Offline — quota figures are last-measured, not live" with the honesty tier downgraded to
Estimated-stale (10 §1 controls: `estimated-stale` > 15m). (3) Handover between local Instances still
works (T1/T2 are file copies, [SPEC.md:280](../SPEC.md#L280)); cross-Provider T3 replay works offline
because priming is local. (4) On reconnect, figures refresh and each card flips back to Measured with a
"refreshed just now" stamp; nothing queued "to send" is sent automatically because nothing was queued —
Fleet (SPEC §11) is LAN/VPN by design and simply shows peers as unreachable.

**Auto (passive).** Staleness labelling, offline banner, refresh on reconnect, harness-update check
deferred (never blocking, 10 §13 `check-failed-offline` is neutral). **Click-confirmed.** Every
launch/handover as usual; login flows that need the network are surfaced as vendor steps the user
retries, never automated ([PROPOSAL.md:43](../PROPOSAL.md#L43)).

**Failure branch.** A login expires mid-trip: the Account card shows `expired` with the vendor-login CTA
greyed "needs network" rather than a spinner that never resolves. If the user ignores staleness and
launches into a nearly-empty window, the forecast pill itself carries the stale label — the number is
never presented as live.

**Delight moment.** The app is *calmer* offline: same board, honest labels, zero error spam. **Features
required.** 10 §1 staleness states, §13 offline states, §8 account health; SPEC local-first posture
([SPEC.md:23](../SPEC.md#L23)) does the heavy lifting. **Emotion + time saved.** Travel anxiety → "it just
works, and says what it does not know." `[inferred]` saves no minutes directly; it saves the trip's first
hour of tooling distrust.

## W9. Team handoff — export pack to a colleague

**Trigger.** Priya goes on leave; Sara's colleague must reproduce a bug. The task *content* must move to
another person even though Accounts never do. Prior framing risked over-blocking this: "sharing profiles
or pools between people is out of scope" ([PROPOSAL.md:45](../PROPOSAL.md#L45)) is a non-goal about
Accounts, not about a user-exported work product. Sara's own journey already needs a "sanitised pack for
internal repro" (07 §9.5).

**Steps in Juggler.** (1) Session panel → [Export pack…] → wizard: step 1 contents (task, decisions,
TODOs, diffs — PACK.md layout, SPEC §7.3 [SPEC.md:297](../SPEC.md#L297)); step 2 redaction preview with
likely secrets highlighted, default-on for export (full review F1.6); step 3 destination: [Save
.juggler-pack…]. (2) User sends the file by their own channel (email/chat/USB) — Juggler has no send
button and no cloud relay ([SPEC.md:369](../SPEC.md#L369)). (3) Colleague: File → [Import pack…] → preview
of exactly what will prime a new Session → picks their own Provider/Account → [Create session from pack].
(4) The imported Session's lineage records "imported pack from <file>" — provenance without any Account
sharing.

**Auto (passive).** Pack assembly from the Transcript IR, secret scanning, lineage stamping.
**Click-confirmed.** Export (after the redaction preview), the manual send (outside Juggler entirely),
and the import-side session creation. Nothing syncs, pools, or grants access; vendor credentials never
enter the pack ([SPEC.md:310](../SPEC.md#L310)).

**Failure branch.** The colleague lacks the Provider or the repo: import still creates the primed Session
for any managed harness (T4 works for any target, [SPEC.md:274](../SPEC.md#L274)) and warns "repo path
differs — primed context references the sender's path." If redaction finds a credential-shaped string,
[Save] stays blocked until it is redacted or the user overrides with an audit-logged confirmation (07 §3
red line).

**Delight moment.** The colleague's first Session opens already knowing the task — the pack, not a
re-told story, did the work. **Features required.** 10 §12 export path, pack redaction (F1.6), §17 audit;
SPEC §7.2 flow 5 export/import ([SPEC.md:290](../SPEC.md#L290)). Positioning note for §23 copy: "share the
work, never the Account." **Emotion + time saved.** Leave-taking anxiety → a verified artefact.
`[inferred]` 20–45 min versus a handover doc plus a "quick call," per transfer.

## W10. End-of-week cost review `[inferred]`

**Trigger.** Friday 17:00. Maya decides whether Max #2 earns its money; Priya invoices; Jonas reports to
sponsors; Alex decides on a $5 top-up. Prior 10 §6 builds the ledger as a feature; nobody designed the
*Friday ritual* that makes it a habit (07 §7.4: "every cost view must end in a decision sentence or
export").

**Steps in Juggler.** (1) Usage → [This week] → matrix: Provider × Account × Project, tokens → Estimated
USD ("API-equivalent value, not your subscription fee," 10 §6), each Account card carrying its manual
subscription fee and one decision sentence: "Max #2 absorbed 3 quota walls this week ≈ $X of API value on
a $200 fee." (2) Anomalies row: top-3 burn Sessions, any Session that burned
> 25% of a window, any unpriced model shown as `—`, never `$0.00`.
(3) [Export CSV] for Priya's per-client invoice (10 §28.3); [Copy summary] for Jonas's sponsor post. (4)
Housekeeping strip: "N Sessions older than 30 days — [Review & archive]" (10 §5), which is also when W2's
parked leftovers and W1's overnight remains get closed out.

**Auto (passive).** Weekly rollups (SPEC §6 [SPEC.md:258](../SPEC.md#L258)), anomaly detection,
decision-sentence templating from recorded walls/handovers (10 §17 events). **Click-confirmed.** Exports,
archiving, any Account change the review inspires. The review never cancels, downgrades, or re-routes
anything itself.

**Failure branch.** Pricing is stale (> 30d) or a model is unpriced: figures carry `stale-pricing` / `—`
and the export's cover note says the total is partial (10 §28.3 failure branch: "never silently sum a
partial total as if complete").

**Delight moment.** Renewal stops being a vibe: one sentence per Account, backed by the week's recorded
walls. **Features required.** 10 §6 ledger, §17 audit-fed decision sentences, §5 archive, §12 export
bundle. **Emotion + time saved.** Sunday-night money anxiety → a five-minute Friday close. `[inferred]`
Priya's 45-minute spreadsheet (07 §3) → < 5 min; Maya/Jonas save the reconstruction, not just the
arithmetic.

## W11. Provider outage day — lab down, failover whole Project

**Trigger.** A Provider's lab is down for hours: every Instance on that Provider fails or hangs at once.
This is not one Session's quota wall (W2) and not one crash (W7); it is correlated failure. No prior
section models it — account health (10 §8) is per-Account login state, not Provider availability.
`[inferred]`

**Steps in Juggler.** (1) Board banner appears passively: "Provider Claude: 4 of 4 Instances failing
since 10:02 — likely Provider-side" (pattern over per-Session errors, not a network probe — Juggler never
proxies traffic, [PROPOSAL.md:42](../PROPOSAL.md#L42)). (2) User clicks the banner → Project failover
sheet: for the affected Project, target Provider/Account rows ranked by headroom and fidelity tier (T3
replay to a different Provider, [SPEC.md:273](../SPEC.md#L273); T2 unavailable cross-Provider, stated
plainly). (3) [Fail over this Project…] → per-Session checklist (the two live Sessions move; finished
ones stay) → [Confirm]. (4) New Instances launch on the surviving Provider primed by replay; original
Sessions remain resumable for the return. (5) When the Provider recovers (first Instance on it next
succeeds), an Inbox row suggests [Return Project to Claude] — the W2 return pattern at Project scope
([SPEC.md:288](../SPEC.md#L288)).

**Auto (passive).** Failure-correlation banner, impact list ("3 Projects, 5 Sessions on this Provider"),
pack staging for every checked Session, recovery suggestion. **Click-confirmed.** The failover itself,
per Project — never a fleet-wide auto-move, which would be auto-rotation at scale and is prohibited
([PROPOSAL.md:62](../PROPOSAL.md#L62)). Deselected Sessions are never touched.

**Failure branch.** The surviving Provider lacks headroom for all checked Sessions: the sheet downgrades
to "fail over 2 of 4 now, park 2 until reset" (composing W2) instead of over-committing a window. If the
outage is actually local (user's network), the banner must say "Instances failing; cause unknown — could
be local network" (W8's honesty rule) rather than naming the lab.

**Delight moment.** A bad morning becomes one sheet and three clicks per Project, with the way back
already staged. **Features required.** 10 §8 extended to Provider-level health, §3 routing at Project
scope, §2 banner; SPEC T3 + return flow; EPICS M4 handover tiers are the substrate. **Emotion + time
saved.** "Everything is down" helplessness → a runbook. `[inferred]` 1–3 hours of manual re-priming per
Project avoided; frequency is low, severity is the whole day.

## 12. Cross-workflow synthesis

**One spine, eleven costumes.** Every workflow rides the same four dependencies, in order: status
emission (SPEC §5.1 — modelled, never emitted; full review F2.5), quota/forecast (10 §1), the Inbox (10
§2), and honest handover tiers (SPEC §7.1). W1–W4 and W11 are impossible before that spine; W5, W6, W9
compose on top of it; W7, W8, W10 harden its edges. This confirms — and sequences — the maximum-depth
report's Phase 1 trio (forecast + inbox + honest handover) from the workflow side.

**New or extended proposal items this pass adds:**

| Item | New / extends | Consumed by | Priority signal |
|---|---|---|---|
| Multi-Session park queue + stagger warning | Extends 10 §9 | W2, W11 | P1, rides the spine |
| Overnight pre-flight + morning digest | New slice of 10 §7 | W1, W3 | P1 `[inferred]` |
| Orphan sweep (detect auto, stop on click) | Surfaces SPEC §5.2 | W7 | P1; maintainer paragraph |
| Provider-level health + Project failover | Extends 10 §8/§3 | W11 | P2; rare, severe |
| Model-aware routing (cheap→strong) | Extends 10 §3 | W6 | P2 `[inferred]` |
| Review pair-switching (diversity ranking) | Extends 10 §11 | W5 | P2, behind dogfood gate |
| Offline staleness copy + stale-pills | Extends 10 §1/§13 | W8 | P1; cheap, copy-level |
| Weekly review ritual + decision sentences | Ritual over 10 §6 | W10, W4 | P1 habit surface |

**Guardrail re-check (all workflows).** None auto-rotates an Account, auto-resumes on a different
Account, auto-kills a process, auto-sends a pack to another person, or proxies traffic. The two
deliberate tensions, stated openly: (a) the maintainer's "automatic… clean ups"
([expected-workflow.md:18](../internal/expected-workflow.md#L18)) is honoured as automatic detection + staged
cleanup, because auto-kill conflicts with [PROPOSAL.md:47](../PROPOSAL.md#L47); (b) W9 ships a
cross-person file while honouring the Account non-goal verbatim — the copy distinction is Accounts vs
work.

**Terminology follow-through.** If §1's ruling is accepted, UI copy passes rename Harness → Provider and
demote Profile to an advanced field; Session/Instance follow ADR-0007. Without that pass, every workflow
above reads twice — once in each vocabulary — which is the current F2.2 conflation repeated.

## 13. Validation and unresolved gaps

Falsify before building beyond the spine (in 07 §11 style):

1. **Overnight demand (W1/W3).** Five dogfood diaries, one week: do the 01:14 waiter and the 15-Session
morning recur at the modelled rates? If not, W1 shrinks to a digest setting. 2. **Arbitrage economics
(W6).** Price two real features both ways (all-strong vs cheap→strong) including replay cost; the
workflow survives only if the split saves ≥ 30% of burn. 3. **Pair-review demand (W5).** Interview signal
only today; keep behind 10 §11's diary gate. Do not build ranking first. 4. **Outage frequency (W11).**
Log Provider-correlated failures passively for a beta cycle before building the failover sheet; the
banner alone may be sufficient value. 5. **Time-saved figures.** All estimates above are `[inferred]`
against named manual workarounds; re-measure with the 10 §28 scripted walks (W3) and 07 §11 diaries (W1,
W4, W10).

Unresolved evidence, named plainly: no competitor was observed offering overnight digests, Project-scope
outage failover, or cheap→strong staged handover in the prior parts (04–06), so W5, W6, and W11 carry no
competitor URL by design — they are extensions of Juggler's own wedge (named multi-Account, quota,
handover), and that absence is exactly why they are marked `[inferred]` rather than asserted as table
stakes.

*End of Pass 2 §07. Claims cite repo `file:line`, a prior-report section, or are marked `[inferred]`; no
workflow restates the prior report's text.*

# Pass 2 — Proposal Delta and Plan Updates (Synthesis)

> Second pass, synthesis-ready. Canonical input:
> `docs/internal/expected-workflow.md:1` (21 lines, maintainer example).
> Prior research cited, not restated: Maximum-Depth Report
> `docs/review/AI-Juggler-Maximum-Depth-Report-and-Proposal.md`
> (6,739 lines; cited as MD §/Part), `docs/review/deep/`
> 10-product-proposal (cited as D10 §), and
> `docs/review/full-competitive-ux-review.md` (cited as FCUR §).
> Baseline: `docs/PROPOSAL.md:1`, `docs/SPEC.md:1`,
> `docs/plan/EPICS.md:1`. Pass-2 siblings 01–07 in this folder
> are the evidence base; this file only synthesises their deltas.
> Terminology: **Provider / Account / Instance** per
> `docs/internal/expected-workflow.md:11`; legacy terms appear only in
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
`docs/internal/expected-workflow.md:5`–`:7`, `:14`, `:17` makes
*switching, seeing, and cleaning up* the product, with quota
as one input to all three. §§1–2 enact that re-centring.

## 1. Revised product thesis and positioning

### 1.1 Thesis (replaces PROPOSAL §1 framing)

Prior thesis (`docs/PROPOSAL.md:10`–`:16`): a supervisor that
does profiles/parallelism, quota dashboards, and handover.
That ordering is the error pass 2 corrects: it lists quota
second and handover third as if they were peers, when the
maintainer workflow makes handover the *daily motion*
(`docs/internal/expected-workflow.md:7`, "as many times I want …
back and forth") and quota merely its trigger
(`docs/internal/expected-workflow.md:5`, "if I ran out of credits").

**Revised thesis.** AI Juggler is the switching layer for
AI coding work: it lets a developer start on any Provider
Account, exhaust it, and continue the *same work* — chat,
rules, project state — on any other Account or Provider,
unlimited times in both directions, without manual
configuration (`docs/internal/expected-workflow.md:6`,`:9`), while
always showing which Provider/Account/model is running
where and owning every process an Instance spawns until
it is verifiably gone (`docs/internal/expected-workflow.md:13`–`:18`).

Three load-bearing differences from the prior thesis:

1. **Switching is a loop, not an event.** Prior handover is
   A→B→A (one return; `docs/SPEC.md:288`; D10 §9 reset queue).
   The maintainer requires an unbounded chain (§2, F-N1/F-U1;
   evidence: pass2/02 §§2–6, pass2/01 R5 = Missing).
2. **Visibility is a surface, not a dashboard page.** The
   right inspector on selection (`docs/internal/expected-workflow.md:14`)
   — not the Overview page (`docs/SPEC.md:345`) — is where
   quota, resources, children, and history compose (§2, F-N3).
3. **Ownership outlives the session.** Processes, ports, and
   dev services an Instance spawned are Juggler's problem
   after close (`docs/internal/expected-workflow.md:17`–`:18`); prior
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
  (`docs/internal/expected-workflow.md:5`–`:6`); pass2/07 W5, W6, W9.
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
  a dev server holding a port (`docs/internal/expected-workflow.md:17`).
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
  `docs/internal/expected-workflow.md:14`–`:15`.
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
  (`docs/internal/expected-workflow.md:13`); pass2/07 W3 morning triage.
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
  (`docs/internal/expected-workflow.md:7`; `docs/SPEC.md:288`).
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
`docs/internal/expected-workflow.md:13`), and feed *switch pre-staging*
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
| D-3 Templates/Prompts Library | §16, Could, RICE 280 | **Cut from roadmap; community/plugin surface** | No maintainer-workflow sentence supports it (`docs/internal/expected-workflow.md:1`–`:21` silent); pack (F-N1) already carries prompts in context | Pack users export/share packs repeatedly (W9 signal) |
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
   `docs/internal/expected-workflow.md:11`. Add subscription-expiry to
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
(`docs/internal/expected-workflow.md:9`) — the word itself is the burden;
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
   `docs/internal/expected-workflow.md:5`–`:18`? If not, which pillar
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



# Pass 2 — Critic Crosschecks (09)

> Scope: critic only. No new design. Checks parts 01–08 against every
> non-blank line of `docs/internal/expected-workflow.md`, against each other,
> and against their own cited evidence. Line counts verified by
> `wc -l docs/review/pass2/*.md` (01:502, 02:567, 03:529, 04:505,
> 05:549, 06:600, 07:524, 08:505; expected file: 21 lines).

## 1. Coverage check — every line of the expected workflow

Non-blank source lines are 1, 3, 5, 6, 7, 9, 11, 13, 14, 15, 17,
18, 20. Verdicts below reuse 01 §2 (R1–R14) and test whether
02–08 actually close the gap 01 named, not merely restate it.

| Line | Demand (short) | 01 verdict | Closed by | Critic verdict |
|---|---|---|---|---|
| 1 | Example only; derive others | R1 Partial | 07 (W1–W11), 06 (P1–P10) | Covered, unvalidated |
| 3 | Many tools/subs/models; quota-expiry tracking | R2 Partial | 04 I-05, 06 P1–P2, 02 §7 | Partial — see C-4 |
| 5 | Start one tool; switch on exhaustion, incl. same-tool | R3 Partial | 02 §§2–5, 07 W2 | Covered (design) |
| 6 | History/context/rules carry; zero manual config | R4 — | 02 §4 pack, 06 P5 | Partial — see U-3 |
| 7 | Unlimited back-and-forth | R5 Missing | 02 §§2, 6, 10 | Covered (design), unproven |
| 9 | Manual profiles too complex; tool absorbs it | R6 — | 03 §§3–5 | Covered (design) |
| 11a | Names: Providers, Accounts, Instances | R7 — | all parts §1 | Conflicted — see C-1 |
| 11b | Smart onboarding: scan, recommend, few options | R8 Partial | 03 §§3–6 | Covered (design) |
| 13 | Every session: provider, account, model, quota | R9 Partial | 04 I-01–I-11 | Partial — see C-5 |
| 14 | Right section: stats, actions, switch; usage/history/quality/perf. | R10 Missing | 04 §5, 01 §3.1, 08 F-N3 | Partial — see C-6 |
| 15 | CPU/RAM per session; subtasks + subprocesses tracked | R11 Partial/split | 04 §5, 05 §§5–8 | Partial — collectors unbuilt |
| 17 | Crashes leave zombies: CLIs, terminals, dev services | R12 — | 05 §§2, 5–7 | Covered (design) |
| 18 | Track + automatic and manual cleanup, verified | R13 Missing + collision | 05 §§9–10, 01 §3.2 | Blocked — see C-2 |
| 20 | Research other developer problems (forums, GitHub) | R14 Partial | 06 P1–P10, 07 W1–W11 | Partial — see U-1, U-2 |

### 1.1 Lines with no owner after pass 2

- Line 3, "quota is expiring": every part models window reset
  (04 I-05 countdown, 06 P2), none models subscription/plan
  expiry or renewal-date warning. 01 R2 named this; no part
  picks it up. Owner: none. Assign to 04 (indicator) + 02 §7
  (ranking input) before build.
- Line 14, "quality": 04 §6 honestly refuses a stuck score and
  marks context % often unknowable, but no part defines the
  quality rollup 01 R10 required. 08 F-N3 still lists it.
  Owner: none with a metric. See C-6.
- Line 15, "subtasks": 05 owns subprocesses; the agent-subtask
  tree (01 §3.3) has no owning part. 04 shows a badge (I-08),
  not a tree. Owner: none.

## 2. Contradictions between parts

### C-1. Provider/Account/Instance rename depth disagrees

- 01 §4: full rename map (UI/domain/DB/docs), cost S/M, plus
  building the Instance entity the tab promised.
- 07 §1: "rename-in-copy-only" for Provider vs Harness.
- 02 §1: Provider = `HarnessId`, Instance = `SessionRecord` +
  `InstanceKind` (`model.rs:47-50,142-157,386-416`).
- 08 §3: 3-phase migration, ADR-0008..0012.
Copy-only (07) cannot coexist with the DB/domain rename (01,
08). Canonical ruling needed; critic recommends 01/08 (rename
once) and treats 07's copy-only as superseded. Until ruled,
every part's §1 mapping is provisional.

### C-2. Automatic cleanup: three incompatible policies

- `docs/SPEC.md:26`: click-bound automation, one auto-restart
  exception. All parts claim to honour it.
- 01 §3.2 / R13: narrow opt-in exception *or* reframe
  "automatic" as one-click pre-verified (decision D-C1).
- 05 §§9–10: Tier 0–3 opt-in tiers with fail-closed
  `ProcessIdentity` verify-before-kill — broader than a
  "narrow exception" once Tier 2+ is enabled.
- 07: passive invariant throughout — no auto-kill, no
  auto-rotate, no auto-resume-cross-account in any workflow.
05's tiers and 07's invariant cannot both ship as written.
This is the same D-C1 the input listed as unresolved; pass 2
widened, not settled, it. No auto-cleanup design past the
manual flow is implementable until the maintainer rules.

### C-3. Instances tab: delete vs build

Prior research proposed deleting the Instances tab (deep 01
B-08, FCUR F2.2, cited in 01 Finding 1). The maintainer
mandates Instances (`expected-workflow.md:11`). 01, 04, 05,
08 all now build on Instance, but 04 §1 also flags
Instance ≡ Session collapse (`SessionInfo.ts:3`,
`WorkflowRail.tsx:295`). Parts agree with the maintainer;
the prior deletion proposal must be explicitly withdrawn in
08's doc updates, or implementers will find both directives.

### C-4. Quota centrality: thesis vs evidence

08 §1 demotes the quota-centred thesis (`PROPOSAL.md:10`,
D10 §25 RICE 600) in favour of switch/see/clean. But 06 —
the only part with external evidence — ranks quota walls
(P1) and reset distrust (P2) as top pains, and 02 §7 makes
quota a ranking input. Demoting quota in positioning while
depending on it in ranking and indicators is not a
contradiction if stated as "quota is a signal, not the
product"; 08 does not state it that cleanly. Risk: roadmap
resequencing (08 §4) under-funds the quota read-model that
04 I-05/I-06 and 02 §7 both require.

### C-5. Running model per Instance is specified but unsourced

04 I-03 and line 13 require the currently running model per
session. 01 already warned: `model.rs:164` is a launch
overlay only; no per-Provider telemetry source is verified.
02, 07, 08 reuse the field without adding a source. The
indicator spec is renderable and permanently empty until a
per-Provider source is proven. Mark I-03 "source unverified"
in the canonical spec (see §4).

### C-6. Right inspector promises vs honest signals

04 §6 (honest table: tokens/min measurable, error-rate
partial, stuck score refused, context % often unknowable)
is the most candid section in pass 2. 01 §3.1 (Snapshot
Panel) and 08 F-N3 (Right Inspector PRD) both list quality
and performance rollups without importing 04 §6's refusals.
If 08 is what gets built, the refusal is lost. Canonical
home for the honesty constraints must be 04 §6, referenced
— not restated — by 01 and 08.

### C-7. Adopt-in-place vs managed-dir invariant

03 flow (f) "Just track it" (adopt in place, ≤3 clicks)
conflicts with the managed-dir invariant (`SPEC.md:400`),
and 03 itself flags that it "needs its own ADR or stays
cut", plus the FCUR F4.1 credential byte-copy contradiction
must be ruled before the scan widens. 08's ADR list
(0008..0012) must name this ADR explicitly; a generic ADR
slot is not a ruling. Until ruled, flow (f) is cut by
default — 03's click budget for (f) should not be quoted
as a commitment.

### C-8. Switch UX: one click vs wizard vs click budgets

02 §8 resolves critic C4 with a 1-click-vs-wizard decision
tree. 03 independently budgets clicks per flow (a≤5 … f≤3)
and 07 specifies "exact clicks" per workflow. Three click
accounting schemes, no shared definition of what counts as
a click (staging? verification? launch?). Minor but real:
acceptance tests in 02 §10 cannot be compared with 03 §9
metrics until one counting rule is canonical (recommend
02 §8).

## 3. Unsupported or under-supported external claims

Source for this section: spot-read of 06's URL body (60+
links: GitHub issues, dev.to, Medium, OpenAI Community, HN)
and the unresolved lists of 01–08. 06 separates verified vs
inferred in-file; the failures below are where later parts
consume an inferred claim as if verified.

- U-1. Reddit: no live thread bodies fetched (search
  snippets only) — 06's own unresolved. Any persona or
  frequency claim resting on Reddit is snippet-grade.
- U-2. Personas: deep 07 personas remain composites (01
  R14); the forum/issue ethnography (R14/D-W6) was not
  executed. 07 (pass 2) builds W1–W11 on those composites
  and marks time-saved `[inferred]` throughout §13.
- U-3. Rules/context carry-over: 06 P5 evidences duplication
  pain (`CLAUDE.md`/`AGENTS.md`, dev.to on v2.1.277), but
  "rules carry automatically" across harnesses (line 6)
  has no verified cross-tool source; 02 §3's per-artifact
  Copy/Translate/Regenerate/Drop matrix is design, not
  evidence.
- U-4. Gemini/OpenCode: quota API parity with Claude/Codex
  unverified; ZCode-parity claim inferred (06 unresolved).
  Yet 02 scores Gemini/OpenCode F0 pack-only and 07 W11
  (provider-outage failover) assumes cross-provider resume.
  W11, W5 (pair-review) and W6 (cheap→strong arbitrage)
  are marked `[inferred]` with no competitor evidence in
  04–06 — do not build before dogfood/interviews and
  outage logging (07 §13 agrees; 08 must not promote them).
- U-5. Codex numbers: Estimated-window accuracy untested
  against a live account (06); the "63%-but-unusable"
  binding-window report (OpenAI Community, 06 P2) is a
  single community thread promoted to a trust-failure
  cluster. Directionally plausible, n=1.
- U-6. Wrong-account billing: silent headless billing
  (`claude-code #72552`, `#74217`, 06 P6/F2) is the best-
  evidenced new claim in pass 2 (two upstream issues) —
  supported. But its promotion to "P0 per-Instance identity
  assertion" is a product judgement, not an external fact;
  08 should cite it as such.
- U-7. Worktree collisions: "hit 3x in one sprint" rests on
  one field note (`murrorastro`) plus a SKILL.md — real,
  but frequency is anecdotal. P0 severity is arguable;
  frequency is not established.
- U-8. ZCode: prime path + SQLite-clone safety unproven
  (`SPEC.md:282`); per-account login probe depends on the
  HOME-isolation experiment (`research/zcode.md:72`) being
  re-run per release. All ZCode cells in 02 §10 stay
  capped F1/F0 until those pass.
- U-9. cc-switch: DB schema version must be re-verified
  live before 03 S4 reads beyond existence/count. 03's
  scan design degrades correctly; any part quoting
  cc-switch contents (not counts) is unsupported.

## 4. Duplicated features — canonical home

Each feature below is designed in two or more parts. The
canonical home is the part with the deepest spec; other
parts must reference, not re-specify. Divergences found
are noted; none were reconciled inside the parts.

| Feature | Parts | Canonical home | Divergence to fix |
|---|---|---|---|
| Portable context pack | 02 §4, 07 W9, 08 F-N1 | 02 §4 (`state.json` fields) | 07 W9 reframes export vs `PROPOSAL.md:45` non-goal; keep framing in 07, fields in 02 |
| Switch-back / divergence | 02 §6, 01 D-W1, 08 F-N5 | 02 §6 (resume + delta, 3-way reconcile) | 01 D-W1 is the requirement; 08 is the PRD shell only |
| Cleanup Centre | 05 §9, 01 §3.2, 08 F-N2, 07 W7 | 05 §§9–10 | Policy tiers vs 07 invariant — C-2 |
| Right inspector / snapshot | 04 §5, 01 §3.1, 08 F-N3 | 04 §§5–6 | Quality promises — C-6 |
| Indicator system | 04 §§3–4, 08 F-N4 | 04 §§3–4 (I-01–I-11) | 08 must not re-rank fields |
| Onboarding scan + recommend | 03 §§3–5, 06 P3–P4 | 03 §§3–5 | 06 is evidence, not a second spec |
| Quota display + ranking | 06 P1–P2, 04 I-05/06, 02 §7 | 04 (display), 02 §7 (ranking) | Pricing table missing (F1.2); cost stays Est/hidden |
| Zombie/orphan detection | 05 §§5–7, 06 P7, 01 §3.2 | 05 §§5–7 (spawn ledger) | No PGID/`setpgid` exists in `crates/` today; ledger is greenfield |
| Terminology mapping | every part §1 | 01 §4 (pending C-1 ruling) | 8 near-identical maps; delete 7 after ruling |
| Destination ranking | 02 §7, 03 §5, 07 W2/W11 | 02 §7 | 03 §5 ranks accounts at setup; keep setup ranking in 03, switch ranking in 02 |

## 5. What pass 2 still does not prove (inherited)

These were unresolved on entry and remain unresolved;
pass 2 designed around them honestly in most cases.

- Live dogfood: Claude 2.1.296 × project shapes, A→B→A
  continuity ≥80% (`PROPOSAL.md:144`) not run. Design only;
  no code changed in any part.
- Upstream emission gap: `WaitingUser`/`Stuck`/`QuotaBlocked`
  modelled (`model.rs:188-207`) but never emitted; the
  rendering contract (04) assumes M2-5 (`EPICS.md:79`).
- Missing read-model: `SessionInfo` lacks account_label /
  model / quota / tokens; no pricing table (F1.2); no
  sysinfo process-tree collector (`SPEC.md:231`); no
  git-status poller (04 I-09).
- No runtime, browser, or screen-reader verification of
  any UI spec in 03, 04, or 05. Contrast, sparkline
  legibility, and density breakpoints are code-read only.
- Windows: Job Object design deferred to M7-5 (05); only
  a trait seam exists. Port probe crate (`netstat2` vs
  alternative) unbenchmarked; sysinfo 0.39 has no portable
  listeners.
- Codex cross-account rollout resume unproven against the
  real binary (02); Gemini/OpenCode adapters do not exist.

## 6. Critic verdict

Pass 2 closes the three structural gaps 01 named — switch
graph (02), snapshot/inspector (04), leftover inventory
(05) — as designs. It does not close the two maintainer
decisions that gate build: D-C1 cleanup policy (C-2) and
the terminology rename depth (C-1). It weakens its own
evidence base wherever 07/08 consume 06's inferred claims
(U-4, U-5, U-7) without the `[inferred]` tag. Recommendation:
rule C-1 and C-2, adopt §4's canonical homes (deleting the
seven duplicate terminology maps), mark 04 I-03 and all
quality rollups "source unverified", and keep W5/W6/W11
and flow (f) out of the roadmap until their named
validations run. Nothing in pass 2 requires new code
before those rulings; everything in it requires them
before commitment.

# Appendix — Preserved Unresolved Items (verbatim union, pass 2)

> Source: caller-preserved unresolved list plus prior results 1-9.
> No item below was closed by pass 2; ordering groups duplicates
> without deleting any preserved wording.

- D-C1 automatic-cleanup scope needs maintainer ruling (narrow opt-in exception vs one-click verified) before auto-cleanup can be designed past manual flow
- Running-model telemetry per Instance unmodelled in code (model.rs:164 is launch overlay only) - source per Provider unverified
- Forum/issue ethnography (R14/D-W6 research) not yet executed; deep 07 personas remain composites pending validation
- ZCode prime path + SQLite-clone safety unproven — capped F1/F0 until §10 cells pass (docs/SPEC.md:282)
- Codex cross-account rollout resume unproven against real binary; Gemini/OpenCode adapters do not exist (F0 pack-only)
- Live dogfood (Claude 2.1.296 x project shapes, A->B->A continuity >=80% per docs/PROPOSAL.md:144) not run — design only, no code changed
- cc-switch DB schema version must be re-verified live before S4 reads beyond existence/count (design degrades to existence-only until then)
- ZCode per-account login probe depends on HOME-isolation experiment in research/zcode.md:72 being re-run per release
- Adopt-in-place ('Just track it', flow f) conflicts with managed-dir invariant SPEC.md:400 — needs its own ADR or stays cut; credential byte-copy contradiction FCUR F4.1 must be ruled before scan widens
- Upstream emission gap (full review F5.2): WaitingUser/Stuck/QuotaBlocked modelled model.rs:188-207 but never emitted — spec assumes M2-5 EPICS.md:79 lands; rendering contract ready, data not yet live
- Missing read-model fields/pricing: SessionInfo lacks account_label/model/quota/tokens; pricing table does not exist (F1.2) so I-06 cost stays Est/hidden until built; sysinfo process-tree collector (SPEC.md:231) and git-status poller (I-09) not implemented
- No runtime/browser/screen-reader verification — contrast, sparkline legibility, density breakpoints are code-read specs only
- Windows Job Object design deferred to M7-5; only trait seam specified
- netstat2 vs alternative PortProbe crate not benchmarked; sysinfo 0.39 lacks portable listeners
- SPEC automation-policy amendment wording for Tier 1/2 opt-in needs maintainer approval before implementation
- No live Reddit thread bodies fetched (search snippets only)
- Gemini quota API parity with Claude/Codex unverified; ZCode-parity claim inferred
- Codex Estimated-window accuracy untested against a live account
- W5 pair-review, W6 cheap->strong arbitrage, W11 provider-outage failover marked [inferred] with no competitor evidence in prior parts 04-06; need dogfood/interviews and outage logging before building (07 §13)
- Time-saved figures are [inferred] vs named manual workarounds, not measured; falsify via 07 §11 diaries and 10 §28 scripted walks (07 §13)
- Maintainer decisions D1-D7 (08 §5) require human choice — defaults recommended, not yet locked
- No new external URLs added by 08 by design; synthesis relies on pass2/01-07 primary evidence — spot-check sibling citations if meeting challenges them
- D-C1 cleanup policy (critic C-2) and rename depth (critic C-1) need maintainer ruling before build; adopt-in-place flow (f) cut until ADR rules SPEC.md:400/FCUR F4.1
- Quality rollup (expected-workflow.md:14) has no metric or owner; plan/subscription expiry and agent-subtask tree also ownerless (09 §1.1)
- Ethnography/persona validation, live dogfood >=80% (PROPOSAL.md:144), ZCode/Codex binary proofs, cc-switch live schema re-verify not run
