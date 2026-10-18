# Pass 2 — Expected Workflow vs Researched Proposal: Gap Audit

> Second pass. Canonical input: `docs/expected-worfklow.md` (21 lines).
> Prior research under audit: `docs/review/AI-Juggler-Maximum-Depth-Report-and-Proposal.md`
> (6,739 lines, 13 deep parts) and `docs/review/full-competitive-ux-review.md`
> (1,095 lines, cited as FCUR). Baseline docs: `docs/PROPOSAL.md:1`,
> `docs/SPEC.md:1`, `docs/plan/EPICS.md:1`. This file cites the prior
> reports by part/section, then extends. It does not restate them.
> Terminology rule for this file: **Provider / Account / Instance**
> per `docs/expected-worfklow.md:11`; legacy terms are quoted only
> when citing code or prior text.

## 0. Verdict in one paragraph

Prior research optimised the wrong centre of gravity. It made
quota-first, handover honesty, and the needs-attention inbox the
product (Maximum-Depth front matter, Executive decisions 1–6),
while the maintainer's example workflow centres three things the
prior reports under-weight or contradict: (a) unlimited,
reversible switching as the daily motion, not a one-way handover
(`docs/expected-worfklow.md:7`); (b) a single right-hand snapshot
surface for any selected session or provider
(`docs/expected-worfklow.md:14`); and (c) lifecycle ownership of
everything an instance spawns, including post-close zombies and
dev services (`docs/expected-worfklow.md:15`–`:18`). Most sharply,
prior research proposes deleting the Instances tab
(deep 01 §B-08; FCUR F2.2) — the exact noun the maintainer mandates
at `docs/expected-worfklow.md:11`. Pass 2 therefore reframes, not
repeats: switching is a loop, the detail panel is the product, and
process ownership is a first-class workflow.

## 1. How to read the traceability matrix

- **Requirement** quotes or tightly paraphrases one sentence of
  `docs/expected-worfklow.md`, with its line number.
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
| Requirement | "This is just one of the expected workflow … there might be others derived from this, competition or other ways." (`docs/expected-worfklow.md:1`) |
| Prior coverage | Maximum-Depth front matter frames itself as answering fixed part questions (`docs/review/AI-Juggler-Maximum-Depth-Report-and-Proposal.md:21`); deep 10 §28 validates only three pre-written journeys. No derived-workflow method is proposed. |
| Code/SPEC coverage | `docs/SPEC.md:316` scopes UX to v0.1 screens; `docs/plan/EPICS.md:24` fixes milestones M0–M8 with no discovery track. |
| Verdict | **Wrong-emphasis** — prior research treats the example as a checklist to audit against, not a seed to generate workflows from. |
| Delta needed | Add a derived-workflow backlog (§5 of this file) owned alongside the roadmap; every derived flow gets the same traceability treatment as R2–R14 before it can enter an epic. |

### R2 — Many tools, subscriptions, harnesses, models; quota-expiry tracking (line 3)

| Field | Evidence |
|---|---|
| Requirement | "I often use multiple tools, multiple subscriptions, each one has their own harnesses and models … tracking if one subscription quota is expiring" (`docs/expected-worfklow.md:3`). |
| Prior coverage | Strongest prior area: Maximum-Depth Executive decision 1 (quota-first) and deep 09 Part Q (Q1–Q9) design the quota engine; FCUR F6.1 reorders quota before adapters. |
| Code/SPEC coverage | Domain: `crates/juggler-core/src/model.rs:145` (`Account`), `:173` (`Profile`), `:277` (`QuotaWindow`); quota feed specced at `docs/SPEC.md:191`; milestones still order quota at M3 in `docs/SPEC.md:453`. |
| Verdict | **Partial** — covered in depth for Claude, but "expiring" is modelled as window reset, not subscription/plan expiry; multi-model identity per session is absent (see R10). |
| Delta needed | Split two clocks in research and UI: (1) quota-window reset (5h/weekly/credits) and (2) subscription renewal/expiry per Account. Prior reports conflate them under "quota"; the maintainer's sentence names subscription expiry as the tracking object. |

### R3 — Start on one tool; switch on exhaustion to another tool, another subscription, or same tool/different subscription (line 5)

| Field | Evidence |
|---|---|
| Requirement | "Work on a project starting with one tool, and then if I ran out of credits … super easily switch to another tool with different subscription or even same tool, sometimes users have multiple subscriptions from same AI lab." (`docs/expected-worfklow.md:5`) |
| Prior coverage | Deep 09 Part H designs T1–T4 tiers; FCUR §Q3 audits handover fidelity (F3.1–F3.5). Three distinct switch kinds in the sentence (cross-tool, cross-subscription, same-tool/multi-subscription) are collapsed into one "handover" verb. |
| Code/SPEC coverage | Tiers defined `docs/SPEC.md:269`; flows `docs/SPEC.md:286`; shipped v0 is a single-file byte-copy (FCUR F3.3, `commands.rs` evidence cited there). |
| Verdict | **Partial** — fidelity is well researched; switch-kind taxonomy and "super easily" (clicks, latency, pre-staging) are not measured anywhere in prior reports. |
| Delta needed | Name three switch kinds as separate product flows with separate success metrics: Switch-Account (same Provider), Switch-Provider (cross-tool), Switch-Instance (resume elsewhere). "Super easily" needs a budget: ≤2 clicks, target pre-staged, no manual path entry — none of which prior acceptance criteria state. |

### R4 — All chat history, context, rules, and guidelines carry over; zero manual configuration (line 6)

| Field | Evidence |
|---|---|
| Requirement | "All the chat history, context, rules and guidelines need to work. I don't want to spend time configuring it out all manually. This is really where the power comes." (`docs/expected-worfklow.md:6`) |
| Prior coverage | Deep 09 H2–H5 covers transcript fidelity and redaction; project rules sync appears as one click with diff preview (`docs/SPEC.md:289`, EPICS M4-6). MCP/settings sync is deferred to P2 (deep 10 §14; FCUR F1.9). |
| Code/SPEC coverage | Pack format `docs/SPEC.md:296`; rules conversion `docs/SPEC.md:289`; parser drops `tool_use`/`tool_result` text (FCUR F3.4, `claude_transcript.rs` evidence). |
| Verdict | **Wrong-emphasis** — prior research optimises for *inspectable* transfer (preview, diff, confirm); the maintainer optimises for *invisible* transfer ("need to work"). Every preview step prior adds is friction the expected workflow counts as failure. |
| Delta needed | Reframe default to zero-touch carry (history + rules + guidelines + MCP/settings) with preview as an optional audit view, not a gate. Research must state what "rules and guidelines" includes per Provider (rules files, memory files, system prompts, MCP config) — currently only rules files are specced. |

### R5 — Unlimited back-and-forth switching (line 7)

| Field | Evidence |
|---|---|
| Requirement | "Also I want to be able to do this as many times I want and switching back and forth as needed." (`docs/expected-worfklow.md:7`) |
| Prior coverage | Return-on-reset is covered as a single A→B→A hop: `docs/SPEC.md:288`, deep 09 H7 lineage, FCUR F1.5 reset queue. No prior section models N switches, switch cost accumulation, or lineage deeper than two hops as a daily pattern. |
| Code/SPEC coverage | Lineage today is one status flip to `HandedOver` (`crates/juggler-core/src/model.rs:205`); deep 03 §6 notes "no pack, no A→B→A chain" in shipped code. |
| Verdict | **Missing** — the defining adverb of the expected workflow ("as many times", "back and forth") has no design, metric, or test in prior research. |
| Delta needed | Promote lineage from a status flag to a chain entity (see §5, D-W1). Acceptance must include a 5-switch same-day fixture with context-integrity checks at every hop; prior H8 test plan (deep 09 §H8) stops at round-trip. |

### R6 — Manual profiles/configurations are too complicated; the tool absorbs the complexity (line 9)

| Field | Evidence |
|---|---|
| Requirement | "Also these profiles, configurations to accomplish this manually are very complicated, this is why we need this tool to take care of it all." (`docs/expected-worfklow.md:9`) |
| Prior coverage | Prior reports expand configuration surface: deep 10 §31 inventories settings for every new knob; FCUR F1.9 adds profile clone/sync checkboxes. Complexity is managed by exposing more of it, carefully. |
| Code/SPEC coverage | `Profile` = account + settings overlay (`crates/juggler-core/src/model.rs:173`); overlay today is model + env only (`crates/juggler-core/src/model.rs:161`). |
| Verdict | **Wrong-emphasis** — prior research treats configuration as a surface to perfect; the maintainer treats it as a burden to remove. The word "profile" itself is part of the burden (see §4). |
| Delta needed | State an anti-configuration budget: a new Account must be launchable with zero user-named settings (defaults derived by scan, R8). Every new knob proposed in deep 10 §31 must name which manual step it deletes, or be cut. |

### R7 — Naming mandate: Providers, Accounts, Instances (line 11, first half)

| Field | Evidence |
|---|---|
| Requirement | "add new provider and account (yes we should call them providers, accounts, and instances of the tool)" (`docs/expected-worfklow.md:11`). |
| Prior coverage | Prior research keeps `harness` / `profile` / `session` throughout and proposes *removing* the Instances tab (deep 01 §B-08; FCUR F2.2; deep 11 rail redesign). The maintainer's noun is treated as the problem. |
| Code/SPEC coverage | Code has `HarnessId` (`crates/juggler-core/src/model.rs:49`), `Account` (`:145`), `Profile` (`:173`), `SessionRecord` (`:389`); `InstanceKind` exists only as Terminal/Desktop (`:325`). ADR-0007 (`docs/adr/0007-instance-centric-information-architecture.md:38`) already defines Instance as the running occurrence — code never grew the entity. |
| Verdict | **Wrong-emphasis (direct conflict)** — §4 of this file gives the full rename map and migration cost. |
| Delta needed | Adopt Provider/Account/Instance as user-facing and domain language; demote harness/profile/session to adapter-internal or legacy-alias status per §4. Reverse the "delete Instances" recommendation. |

### R8 — Fully smart automated onboarding: scan, recommend, few options (line 11, second half)

| Field | Evidence |
|---|---|
| Requirement | "They need to have a fully smart and automated onboarding wizard. It can't have thousands of options, it needs to do most of the figuring out on its own, that means scanning directories, existing providers … and making recommendations. It needs to be smart." (`docs/expected-worfklow.md:11`) |
| Prior coverage | FCUR F2.6 calls onboarding "genuinely the best-built surface" and recommends freezing it; deep 03 audits leaks (E1–E14) and adds guards. Neither extends it to cross-Provider recommendation or new-Provider onboarding mid-life. |
| Code/SPEC coverage | Scan/import/login/summary flow specced `docs/SPEC.md:409`; ADR-0006 (`docs/adr/0006-smart-onboarding-scan-and-import.md:28`) covers Claude only; wizard auto-opens at zero accounts. |
| Verdict | **Partial** — detection exists for one Provider; "making recommendations" (which Account/Provider to add, which to prefer per project, what to import vs create fresh) has no engine, no ranking, and no acceptance test in prior work. |
| Delta needed | Specify a recommender: scan all known Provider footprints, rank candidate Accounts by recency/plan headroom/project affinity, and present one recommended path with alternatives collapsed. Freeze (F2.6) is the wrong verb; the wizard must learn. |

### R9 — Every session labelled: provider, account, model, quota left (line 13)

| Field | Evidence |
|---|---|
| Requirement | "All the sessions should have icons and labels, which provider it is, account, models currently running. Amount of quota left …" (`docs/expected-worfklow.md:13`) |
| Prior coverage | Deep 01 §B-11 and deep 02 §7 redesign status chips; the header Provider chip is static text today (deep 01 §B-06). Quota glance is deferred to a rail footer concept (deep 01 §10 wireframe). No prior spec requires the four-tuple (Provider, Account, model, quota) on every row. |
| Code/SPEC coverage | Session row identity lives in the rail (`WorkflowRail.tsx` per deep 01 §B-11); SPEC screen list `docs/SPEC.md:345` promises quota rings only on Overview, not per-row. Model is stored only as a profile overlay field (`crates/juggler-core/src/model.rs:164`), never as running-model telemetry. |
| Verdict | **Partial** — status labelling is well audited; identity labelling (which Provider/Account/model, with quota) is thinner than the maintainer's "all … should have" bar, and running-model is unmodelled. |
| Delta needed | Make the four-tuple a row invariant with an honesty fallback (`model: unknown` beats omission). Prior chip work (deep 02 §7) should be extended, not replaced: status + identity + quota on one row, text always paired with colour. |

### R10 — Select a session or provider: whole right section becomes stats, actions, switch; snapshot of usage, history, quality, performance (line 14)

| Field | Evidence |
|---|---|
| Requirement | "When you select a specific session or provider, whole right section should come with all kinds of stats, actions to manage, switch … whole goal is to a snapshot of usage, history, quality, performance." (`docs/expected-worfklow.md:14`) |
| Prior coverage | Prior IA splits this across three places: Overview (global dashboard, deep 11), SessionPanel (terminal-centric, deep 02 §5), and Accounts cards (deep 02 Profiles audit). Deep 11 §3 makes the needs-attention inbox the default landing instead. No Provider-detail right panel exists in any prior screen spec. |
| Code/SPEC coverage | SPEC instance panel `docs/SPEC.md:347` lists status/account/resources/tokens/actions; SPEC has no Provider-detail screen at all (`docs/SPEC.md:343` screen list). "Quality" appears nowhere as a measured dimension in SPEC or prior quota design (deep 09 Part Q measures tokens/USD only). |
| Verdict | **Missing** — this is the largest single gap. The maintainer describes one contextual snapshot surface; prior research describes a dashboard plus a terminal with chips. "Quality" and "performance" as per-Instance/Provider rollups are unresearched. |
| Delta needed | Specify the Snapshot Panel (§3.1): one right-hand surface parameterised by selection (Instance or Provider or Account), holding stats, actions including Switch, and four snapshot lenses — usage, history, quality, performance. Prior inbox (deep 10 §2) becomes its triage feed, not the home screen. |

### R11 — Per-tool, per-session resource use; subtasks and subprocesses tracked (line 15)

| Field | Evidence |
|---|---|
| Requirement | "For each tool, session, we should show how much resources it is using, cpu, ram, did it spawn any sub tasks, sub processes, and allow to track those." (`docs/expected-worfklow.md:15`) |
| Prior coverage | Deep 08 §5–§6 treats the process tree as a supervisor correctness problem (kill, adoption, scale); SPEC §5.2 (`docs/SPEC.md:227`) specs per-session RSS/CPU attribution. Subtasks (agent subagents/tasks inside the conversation) are mentioned only as a fidelity risk for handover (FCUR F3.4). |
| Code/SPEC coverage | Descendant query exists (`crates/juggler-supervisor/src/supervisor.rs:347`); footprint learning `docs/SPEC.md:235`. No subtask entity exists; transcript parser drops tool outcomes (FCUR F3.4). |
| Verdict | **Partial, split in two** — subprocess resources: specced, unbuilt, correctly prioritised as M2. Subtask tracking: **Missing** as a user-facing concept; prior research never asks "what is this session's agent doing right now" as a tree the user can inspect. |
| Delta needed | Two trees, one panel: (1) OS process tree with CPU/RAM per node (SPEC §5.2 as written); (2) agent subtask tree parsed from transcript/tool events, linked to the processes it spawned where correlation exists. Prior FMEA (deep 08 §5) covers tree-kill; it never covers tree-*reading* as a daily workflow. |

### R12 — Crashes leave zombie processes: CLIs, terminals, dev services outlive the session (lines 17–18, first half)

| Field | Evidence |
|---|---|
| Requirement | "Sometimes these tools crash, exit, and their subprocesses, cli commands, terminals and dev local services they started end up running long after the session closed, they end up being zombie processes." (`docs/expected-worfklow.md:17`) |
| Prior coverage | Deep 08 F-5 concludes "Our zombies: none … out of scope, document it"; F-9 notes force-kill kills only the direct child so subagents survive a stopped session. The maintainer's scenario — *unmanaged* leftovers (dev servers, terminals the tool opened, processes reparented after a crash) — is explicitly scoped out. |
| Code/SPEC coverage | SPEC's only zombie sentence is re-adoption on app restart (`docs/SPEC.md:224`); supervisor stop ladder `crates/juggler-supervisor/src/supervisor.rs:296` (per deep 08 §5). No orphan inventory, no port/owner attribution, no leftover detection exists. |
| Verdict | **Wrong-emphasis** — prior research answers "do *we* leak zombies?" (no) when the maintainer asks "help me *see and clean* the leftovers my tools leak" (unresearched). These are different products. |
| Delta needed | New capability: Leftover Inventory (§3.2) — after any Instance ends, diff its process/port/terminal footprint against the live system and list survivors with owner, age, and resource cost, including dev services identified by listening port. |

### R13 — Super-easy tracking with automatic and manual cleanups, all verified (line 18, second half)

| Field | Evidence |
|---|---|
| Requirement | "Our tool should have super easy way to track that, offer automatic and manual clean ups, all with verifications …" (`docs/expected-worfklow.md:18`) |
| Prior coverage | Deep 08 §8 target architecture adds "tree-kill" to the stop ladder; FCUR and Maximum-Depth guardrails forbid autonomous state-changing actions (automation policy, `docs/SPEC.md:26`). No cleanup workflow, verification step, or automatic-cleanup policy is designed anywhere in prior reports. |
| Code/SPEC coverage | Automation invariant `docs/SPEC.md:26` permits only passive behaviour plus one opt-in same-profile auto-restart; cleanup of leftovers is neither permitted nor forbidden — it is unaddressed. |
| Verdict | **Missing, with a policy collision** — "automatic clean ups" cannot be bolted on; it needs an explicit, narrow exception designed like the existing auto-restart exception (opt-in, per-Instance, same-owner only, verified post-kill), or the maintainer's "automatic" must be reframed as "one-click, pre-verified". Prior research never surfaces this decision. |
| Delta needed | Maintainer decision D-C1 (§6): automatic cleanup scope. Until ruled, design manual verified cleanup (identify → confirm with consequence → kill tree → re-scan verification → report) and treat any automation as a separate ADR amending `docs/SPEC.md:26`. |

### R14 — Research other developer problems beyond these examples (line 20)

| Field | Evidence |
|---|---|
| Requirement | "I need you really to research other ways and solve other problems for developers with similar cases. Think what kind of problems … research forums, github …" (`docs/expected-worfklow.md:20`) |
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
performance (`docs/expected-worfklow.md:14`). It is one surface
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
(`docs/expected-worfklow.md:17`). The tool must track them and
offer automatic and manual cleanup "all with verifications"
(`docs/expected-worfklow.md:18`).

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
sentence (`docs/expected-worfklow.md:15`). Subtasks — subagents,
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

Mandate: `docs/expected-worfklow.md:11`. Method: adopt the
maintainer's three nouns as the only user-facing and domain
vocabulary; map every existing term exactly once; flag where a
naive rename would break compliance language or stored data.

### 4.1 The three concepts, defined once

| Maintainer term | Definition (proposed canonical) | Absorbs today |
|---|---|---|
| **Provider** | A tool/lab offering whose CLI or app Juggler supervises (Claude Code, Codex, ZCode, future). Owns auth model, quota shape, adapter. | `HarnessId` / "harness" (`crates/juggler-core/src/model.rs:49`); "vendor" in compliance prose (`docs/PROPOSAL.md:60`) |
| **Account** | One subscription/login of one Provider, owned by one person, with an isolated state dir and a plan. Multiple Accounts per Provider is the founding case (`docs/expected-worfklow.md:5`). | `Account` (`crates/juggler-core/src/model.rs:145`) — already correct; keep |
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
   "instances of the tool" (`docs/expected-worfklow.md:11`)
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

Source line 1 invites derivation (`docs/expected-worfklow.md:1`);
line 20 demands it (`docs/expected-worfklow.md:20`). Each workflow
below follows from the stated example by changing one variable.
Prior coverage is cited so derivation never double-counts credit.

### D-W1 — Switch back (return to the original Account/Provider)

Implied by "back and forth" (`docs/expected-worfklow.md:7`).
Prior covers return-on-reset as a suggestion (`docs/SPEC.md:288`;
FCUR F1.5). Missing: return as a *first-class repeated motion*
with delta priming ("while you were away", `docs/PROPOSAL.md:92`)
verified at hop N, not hop 2, and return across Providers
(B→A where A is a different tool), which T3 replay must support
in both directions. Design lineage as a chain with per-hop
fidelity records (extends deep 09 H7).

### D-W2 — Switch mid-subtask (not at a clean boundary)

Implied by "ran out … I want to be able to super easily switch"
(`docs/expected-worfklow.md:5`): exhaustion rarely waits for a
subtask to finish. Prior handover assumes a freezable source
(FCUR F3.3 recommends stop/freeze before copy). Missing: what
carries when a subtask is mid-flight — partial tool state,
running subprocesses (§3.3 tree), uncommitted edits. Needs an
explicit "switch now, reconcile on resume" semantics with the
subtask marked interrupted in the target, never silently dropped.

### D-W3 — Multiple switches per day across one project

Implied by "as many times I want" (`docs/expected-worfklow.md:7`)
at daily scale. Prior personas run 4–20 sessions/day (deep 07)
but switches are events, not a rhythm. Missing: switch cost
accounting (time, context loss, quota spent per hop), a same-day
switch timeline on the Project snapshot, and guardrails against
thrash (suggest staying when headroom is nearly equal). Metric:
switches/day and continuity rating per switch join the success
metrics at `docs/PROPOSAL.md:142`, which today test only one
round-trip (≥80% continuity).

### D-W4 — Parallel same-Provider Accounts on one project

Stated at `docs/expected-worfklow.md:5` ("multiple subscriptions
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
sentence (`docs/expected-worfklow.md:11`) read with daily use:
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
| 1 | Keep and build the real **Instance** entity; reverse the Instances-tab deletion. | deep 01 §B-08; FCUR F2.2 | `docs/expected-worfklow.md:11` |
| 2 | Adopt **Provider / Account / Instance** per §4 (ADR-0014 + v5 migration); Profile becomes internal LaunchRecipe. | Throughout prior (harness/profile/session) | `docs/expected-worfklow.md:11` |
| 3 | Build the **Snapshot Panel** (§3.1) as the primary detail surface for Instance/Account/Provider selection. | deep 11 §3 (inbox as home); deep 02 §5 | `docs/expected-worfklow.md:14` |
| 4 | Ship **Leftover Inventory + verified manual cleanup** (§3.2); re-open deep 08 F-5's out-of-scope ruling. | deep 08 F-5, F-9 | `docs/expected-worfklow.md:17`–`:18` |
| 5 | Rule **D-C1**: automatic-cleanup scope as a narrow, opt-in exception (or explicitly reframe as one-click verified). | Automation policy `docs/SPEC.md:26` (unaddressed case) | `docs/expected-worfklow.md:18` |
| 6 | Model switching as an **unlimited loop**: lineage chain entity, 5-hop acceptance fixture, switches/day metric. | deep 09 H7; `docs/SPEC.md:288` (single return) | `docs/expected-worfklow.md:7` |
| 7 | Split switch into **Switch-Account / Switch-Provider / Switch-Instance** with a ≤2-click, pre-staged budget for "super easily". | deep 09 Part H (one handover verb) | `docs/expected-worfklow.md:5` |
| 8 | Default handover to **zero-touch carry** (history+rules+guidelines+MCP); preview becomes optional audit, not a gate. | deep 09 H5; EPICS M4-6 diff-gate | `docs/expected-worfklow.md:6` |
| 9 | Add **mid-subtask switch** semantics: interrupted subtasks marked and reconciled, never dropped (D-W2). | FCUR F3.3 (freeze assumption) | `docs/expected-worfklow.md:5` |
| 10 | Turn onboarding into a **recommender** (scan all Providers, ranked recommendation, mid-project add ending in a switch, D-W5). | FCUR F2.6 (freeze); deep 03 E14 | `docs/expected-worfklow.md:11` |
| 11 | Enforce the **row four-tuple**: Provider icon, Account, running model (or honest `unknown`), quota left (R9). | deep 01 §B-06/§B-11 (status-only chips) | `docs/expected-worfklow.md:13` |
| 12 | Define **Quality** as operational reliability (finish/fail/blocked, handover/resume success) on the snapshot; ban model-graded scores. | Absent (deep 09 Q measures tokens/USD) | `docs/expected-worfklow.md:14` |
| 13 | Ship the **subtask tree** (§3.3) beside the process tree; "track those" covers both. | FCUR F3.4 (fidelity footnote only) | `docs/expected-worfklow.md:15` |
| 14 | Split **quota-window reset** from **subscription expiry** as two tracked clocks per Account. | deep 09 Part Q (single quota concept) | `docs/expected-worfklow.md:3` |
| 15 | Apply the **anti-configuration budget**: zero user-named settings to launch; each deep 10 §31 knob must name the manual step it deletes. | deep 10 §31; FCUR F1.9 | `docs/expected-worfklow.md:9` |
| 16 | Promote **wrong-Account guard** (affinity) to launch-time confirmation for same-Provider parallel work (D-W4). | FCUR F1.4 (warning) | `docs/expected-worfklow.md:5` |
| 17 | Run the **forum/issue ethnography** and re-validate deep 07 composites before personas rank features (R14). | deep 07 (`[composite]`, §11 pending) | `docs/expected-worfklow.md:20` |
| 18 | Add the **bad-day journey** D-W6 (crash → leftovers → switch → verified cleanup → resume/retire) to dogfood gates. | deep 07 §9; deep 10 §28 (no chained journey) | `docs/expected-worfklow.md:17`–`:18` + `:5` |
| 19 | Re-sequence EPICS: Snapshot Panel + Leftover v1 join the Claude loop phase; they outrank second-Provider breadth. | deep 12 roadmap; `docs/plan/EPICS.md:26` | `docs/expected-worfklow.md:14`–`:18` |
| 20 | Add **switch-cost accounting** (time/context/quota per hop + anti-thrash suggestion) to Usage (D-W3). | deep 09 Q6 (single recommendation score) | `docs/expected-worfklow.md:7` |

**Maintainer decisions requested (blocking, in order):**
D-C1 automatic-cleanup scope (#5); D-T1 terminology ADR-0014
adoption (#2); D-S1 Snapshot Panel as primary surface (#3).
All other corrections proceed under existing guardrails
(`docs/SPEC.md:26`) without new rulings.

*End of Pass 2 audit. Companion evidence: prior reports cited
inline; no prior file was modified by this pass.*
