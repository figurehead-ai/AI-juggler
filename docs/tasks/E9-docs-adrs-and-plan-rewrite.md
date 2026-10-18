# E9 — Docs, ADRs and Plan Rewrite (Stage 1, D-15)

> Epic owner: Docs/Architecture. Status: TODO (all tasks). This epic IS
> D-15 Stage 1: documents only — SPEC, PROPOSAL, EPICS rewrites, ADR-0008
> to ADR-0012 plus Fleet ADR-0013, decision-log finalisation workflow,
> README truth fix, beta-exit criteria, billing-API research log. NO
> product code, tests-as-code, runtime or release changes are authorised
> by any task below; code follows only under separate approval
> ([decision-log.md](../review/decision-log.md#L152) D-15:152-164).
> Settled law, do not re-litigate: D-01 Switch→See→Clean thesis, D-02/D-10
> terminology, D-03 hybrid switch, D-04/D-11 labelled beta, D-05 resume+
> delta, D-06 honest inspector, D-07 measurable quality, D-08 notify-only
> cleanup, D-09 tagline A + subtitle B, D-12 user-entered expiry, D-13
> fleet at launch, D-14 user-chosen pack redaction.
> Rewrite map: pass2/08 §3.1 (SPEC), §3.2 (PROPOSAL), §3.3-3.4 (EPICS/ADR),
> §4 roadmap; corrected by D-13 (fleet NOT schema-only — pass2/08 D-2/D6
> and §4 Phase-4 fleet deferral are superseded) and D-08/D-14 (pass2/08
> §5 defaults D3/D7 are superseded by decision-log wording).
> Every task ends with a per-doc Done checklist; AJ-E910 runs the final
> cross-doc link/term consistency proof. Docs claim only what code or a
> labelled-beta decision supports — never `lossless` for cross-Provider.

### AJ-E901 SPEC §3 domain-model rewrite (Provider/Instance/Switch Graph/ledger) | Status: DONE-EVIDENCED (zcode-primary, 2026-10-13) | Priority P0 | Effort M | Depends: —
**Goal**: Rewrite SPEC §3 so the domain model names Provider/Account/Instance, Switch Graph, spawn ledger and two clocks exactly as D-01/D-02/D-05/D-12 settle them.
**User story**: As a maintainer (P4 Operator), I want one normative model so adapter, supervisor and UI agents stop inventing incompatible entities.
**Requirements**:
1. SPEC §3 SHALL define Provider (= `HarnessId`, [model.rs](../../crates/juggler-core/src/model.rs#L145) area), Account (unchanged), Instance (= `SessionRecord`+`InstanceKind`, model.rs:325/389), Session (conversation inside an Instance only), and Profile demoted to Account-scoped launch-settings overlay (model.rs:173), citing ADR-0008 (AJ-E907) and ADR-0007.
2. The section SHALL add Work/Epoch/SwitchEdge lineage (N-hop, cycles, 2-parent merge, uuid dedupe; pass2/02 §§2,6), spawn-ledger entity (pass2/05 §§2,9) and pack-v2 lineage embedding, each with fields, invariants and `Unknown/Unavailable — reason` states per D-06.
3. Migration SHALL be stated as additive/serialisation-compatible aliases, wire values (`claude-code`/`codex`/`zcode`) unchanged, old DB/JSON round-trips; no code change is specified as done in this task.
4. Done checklist for §3 SHALL list every old term removed, every new entity, and every cross-ref to §§5/6/7/9/11 updated.
**UI/UX reasoning**: Nielsen NN-g #4 Consistency and standards — one noun per concept in every heading, table and diagram label; glossary first-use expansion per D-09 zombie precedent. No UI ships; diagram labels SHALL match ADR-0008 verbatim.
**Performance budget**: Docs only; §3 ≤220 lines (from 64), edit review ≤30 min; 0 ms/0 MB runtime, no poll/scale claim.
**Quality gates**: No `harness` as user noun, no `Profile` as rail noun, no `lossless` cross-Provider claim in §3; maintainer approves the diff.
**Evidence/validation proof**: `grep -n "Provider\|Instance\|SwitchEdge" docs/SPEC.md` output + `git diff -- docs/SPEC.md` stored in PR; pass = all Req.1-3 terms present, old-model-only claims absent, links resolve. Evidence in PR description.
**Visual testing**: Render SPEC.md at 100-col terminal and GitHub preview, 1280×800; capture §3 tables/diagram; pass = no table >4 columns, mermaid renders, no overflow. A11y: headings sequential, alt text on diagram.
**Files to touch**: [docs/SPEC.md](../SPEC.md#L86) (§3 only), [docs/adr/0007-instance-centric-information-architecture.md](../adr/0007-instance-centric-information-architecture.md) (cite only).
**Research basis**: [decision-log.md](../review/decision-log.md#L21) D-02/D-05/D-10; pass2/08 §3.1 item 1, §3.4; pass2/00 §3; pass2/02 §§2,6.
**Handover note**: Do not rename Rust types here — E1 Phase B owns code aliases. Trap: pass2/01 `ADR-0014` numbering is dead; ADR-0008 is canonical (pass2/08 §3.4).

### AJ-E902 SPEC §5 supervisor/ledger + §6 two-clock usage rewrite | Status: DONE-EVIDENCED (zcode-primary, 2026-10-13) | Priority P0 | Effort M | Depends: AJ-E901
**Goal**: Rewrite SPEC §5 (spawn ledger, verified cleanup tiers) and §6 (window vs subscription clocks) so Clean and quota-trigger match D-08/D-12.
**User story**: As a P4 Operator whose closed Instance left a vite server on a port, I want the spec to own that process until verifiably gone, never kill what Juggler did not spawn.
**Requirements**:
1. §5 SHALL specify spawn-ledger recording at launch, ProcessIdentity verify-before-kill, post-close orphan/port detection ≤10 s (pass2/05 §§2,5-9), Tier 0-3 wording exactly per D-08 (notify-only default, Clean-safe opt-in per Account, audit-logged), and the §1 automation-invariant narrow amendment cross-ref to ADR-0010.
2. §5 SHALL preserve no-SIGSTOP ([SPEC.md](../SPEC.md#L225)) and SIGTERM→SIGKILL ladder, state fleet-shaped ownership per D-13 (machine identity on every ledger row), and forbid touching unowned processes.
3. §6 SHALL split window clock (5h/weekly reset) from subscription clock (renewal/expiry, user-entered optional per D-12; absent = `unknown`, never inferred), keep Measured/Computed/Estimated honesty (SPEC.md:259), per-model attribution, and quota-as-switch-trigger incl. pack pre-staging.
4. Done checklist for §§5/6 SHALL enumerate claims deleted (live-tree-only §5, single-clock §6), ADR-0010/0012 links added, and §9/§11 refs repaired.
**UI/UX reasoning**: Nielsen NN-g #5 Error prevention + WCAG 2.2 AA 1.4.1 (never colour-alone): spec SHALL name exact copy `Unavailable — no renewal date entered`, `Notify only`, `Clean safe (this Account)`, and confirmation states for any kill.
**Performance budget**: Docs only; §§5+6 ≤260 lines combined; budgets stated normatively: orphan detect ≤10 s, inspector aggregate ≤200 ms at 50 Instances, ledger write ≤5 ms/spawn (targets for later code, labelled as budgets not measurements).
**Quality gates**: Zero auto-kill without opt-in wording; zero inferred expiry; D-08 per-Account (not per-Provider) storage key stated.
**Evidence/validation proof**: `grep -ni "inferred\|auto.*kill" docs/SPEC.md` reviewed line-by-line in PR (hits only in prohibition sentences); fixture story from pass2/05 §9 quoted as acceptance scenario. Pass = maintainer sign-off on §§5/6 diff.
**Visual testing**: Render §§5/6 at 100-col + GitHub preview 1280×800; capture tier table + two-clock diagram; pass = ≤4-col tables, mermaid renders. A11y: table headers present, no colour-only tier encoding in text.
**Files to touch**: [docs/SPEC.md](../SPEC.md#L214) (§§5-6), [docs/review/pass2/05-process-trees-zombies-and-cleanup.md](../review/pass2/05-process-trees-zombies-and-cleanup.md) (cite only).
**Research basis**: [decision-log.md](../review/decision-log.md#L79) D-08, :113 D-12; pass2/08 §3.1 items 2-3; pass2/05 §§2,9-10; deep/09 §Q1-Q4.
**Handover note**: Trap: pass2/05 §10 tiers are per-Provider in source text — D-08 overrides to per-Account; write per-Account and footnote the override. E5 owns implementation tasks; this task owns only normative wording.

### AJ-E903 SPEC §7 Switching rewrite (Switch Graph, pack v2, divergence) | Status: DONE-EVIDENCED (zcode-primary, 2026-10-13) | Priority P0 | Effort M | Depends: AJ-E901
**Goal**: Retitle and rewrite SPEC §7 Handover as unlimited bidirectional Switching with honest beta fidelity per D-03/D-04/D-05/D-11/D-14.
**User story**: As a P1 Multi-Maxer at a quota wall, I want to switch A→B→A unlimited times and see exactly what copies, translates, regenerates or drops before I confirm.
**Requirements**:
1. §7 SHALL define three switch kinds (Switch-Account, Switch-Provider, Switch-Instance), Switch Graph cycles/N-hop/fork-merge, executor freeze→stage→verify→launch (pass2/02 §5), and preserve click-to-execute invariant verbatim (SPEC.md:263, ADR-0003).
2. §7 SHALL make pack v2 the canonical staging/preview artifact for all tiers (state.json, redaction manifest, lineage; pass2/02 §4), state D-14 verbatim (user-chosen redaction, remembered globally + per-Provider override, policy visible pre-switch, first-run explicit choice), and prohibit silent lossy switches (D-03 wizard triggers enumerated).
3. Switch-back SHALL be resume-original + one summarised delta turn with pack attached; copy-back prohibited; divergence surfaced, never merged silently (D-05).
4. Fidelity SHALL be per-Provider-pair Copy/Translate/Regenerate/Drop matrix with validated vs unvalidated beta badges; the word `lossless` SHALL appear only for T1/T2 same-Provider proven cases, never as a cross-Provider claim.
5. Done checklist for §7 SHALL list retired one-way language, T1-T4 relabelled to fidelity states, and §§3/9/12.1 refs updated.
**UI/UX reasoning**: Nielsen NN-g #1 Visibility of system status + #3 User control/freedom: spec SHALL prescribe one-click same-Provider proven path with undo window, wizard only for cross-Provider/low-fidelity/diverged, exact badge copy `Beta — validated` / `Beta — unvalidated`, shortcut unchanged.
**Performance budget**: Docs only; §7 ≤240 lines; normative budgets: pack assembly <5 s/500 turns, switch ≤2 clicks same-Provider, fidelity lookup 0 added network calls.
**Quality gates**: No auto-execute path described; no secret default imposed (D-14); every tier names its fixture gate from AJ-E910/E2.
**Evidence/validation proof**: `grep -ni lossless docs/SPEC.md` — every hit justified in PR checklist; 5-switch fixture scenario (pass2/02 §6.1) quoted as §7 acceptance. Pass = maintainer approves §7 diff + ADR-0011 link (AJ-E907).
**Visual testing**: Render §7 at 100-col/GitHub 1280×800; capture fidelity matrix + flow list; pass = ≤4-col tables, all states (validated/unvalidated/diverged) present in text, not colour alone (WCAG 1.4.1).
**Files to touch**: [docs/SPEC.md](../SPEC.md#L261) (§7), [docs/adr/0003-automation-policy-compliance-guardrails.md](../adr/0003-automation-policy-compliance-guardrails.md) (cite only).
**Research basis**: [decision-log.md](../review/decision-log.md#L31) D-03/D-04/D-05/D-11/D-14; pass2/08 §3.1 item 4, F-N1/F-N5/F-U1/F-U5; pass2/02 §§2-10.
**Handover note**: Trap: pass2/08 §5 D7 recommended always-withhold redaction — D-14 supersedes it (user choice). Do not copy that default into §7.

### AJ-E904 SPEC §9 UX rewrite (Inspector primary, indicators, Cleanup Centre) | Status: DONE-EVIDENCED (zcode-primary, 2026-10-13) | Priority P0 | Effort M | Depends: AJ-E901
**Goal**: Rewrite SPEC §9 so the Right Inspector is the primary detail surface and Instances stay a first-class tab.
**User story**: As a daily user, I want selecting anything to fill the right section with usage, history, quality signals, processes and actions, per the settled inspector scope.
**Requirements**:
1. §9 SHALL specify the 6-tab Inspector (Overview, Usage, History, Quality/Performance, Processes, Actions) for Session, Provider, Account, Instance and Project selections, with honest-fields wording (`Unavailable — <reason>`) per D-06 and signals-only quality per D-07.
2. §9 SHALL reverse the merge-Instances-away direction: Instances tab stays (D-02, expected-worfklow.md:11); indicator row grammar (Provider, Account, model, state, quota, cost, resources, children, git) SHALL be normative with text+icon redundancy, never colour alone.
3. §9 SHALL add the Cleanup Centre, Leftover Inventory, subscription-expiry warning (user-entered date, D-12) and the two positioning strings (D-09) in the About/home copy.
4. Done checklist SHALL list every §9 screen whose spec changed and every deep/11 wireframe superseded or adopted.
**UI/UX reasoning**: NN-g #1 visibility + #4 consistency; WCAG 1.4.1. Exact copy, shortcuts (⌘1..⌘3 tabs, inspector focus order) and states (populated/empty/loading/error/stale/offline) SHALL be written as testable text, not adjectives.
**Performance budget**: Docs only; §9 ≤300 lines; normative budgets copied from E4/E8 (inspector <200 ms, rail <300 ms at 50 rows) labelled as budgets.
**Quality gates**: No screen described without its five states; no composite quality score anywhere; maintainer approves diff.
**Evidence/validation proof**: `git diff -- docs/SPEC.md` §9 section + term grep in PR; pass = Req.1-3 present, old Overview-only language removed.
**Visual testing**: Render §9 at 100-col and GitHub 1280×800; capture Inspector tab table and indicator grammar table; pass = ≤4-col tables, states enumerable from text alone.
**Files to touch**: `docs/SPEC.md` (§9), `docs/review/deep/11-ux-redesign-screen-spec.md` (cite only).
**Research basis**: decision-log D-06/D-07/D-09/D-12; pass2/04 §§3-6; pass2/08 §3.1 item 5.
**Handover note**: E4 owns implementation tasks; this task owns normative UX wording only. Do not invent fields the read-model cannot supply — mark them Unavailable with reason.

### AJ-E905 SPEC §11 Fleet rewrite (Fleet at launch) + §12.1 onboarding generalisation | Status: DONE-EVIDENCED (zcode-primary, 2026-10-13) | Priority P0 | Effort M | Depends: AJ-E901, AJ-E902
**Goal**: Bring Fleet forward to launch scope and generalise onboarding beyond Claude-only, exactly as D-13 and the smart-scan design settle.
**User story**: As a maintainer, I want the spec to admit Fleet is a launch architecture constraint so no local design has to be retrofitted for multi-machine.
**Requirements**:
1. §11 SHALL specify Fleet at launch in beta: machine identity/pairing, transport and security model, remote Inspector, cross-machine process ownership, offline/conflict behaviour, and the security review as a beta-exit gate (detail owned by E7 tasks; §11 states the contract).
2. §11 SHALL state what Fleet never does: no credential sync, no login automation, no traffic proxying (invariants preserved per machine).
3. §12.1 SHALL generalise the Claude-only scan to the cross-Provider recommender: Found/Likely/Guessed inference, set-level proposals, six flows (a)-(f) with click budgets (pass2/03 §§3-6), and isolation verification.
4. Done checklist SHALL list superseded deferral language (schema-only/post-1.0 Fleet) removed wherever it appears in SPEC.
**UI/UX reasoning**: NN-g #2 match with real world (Providers/Accounts/Instances nouns throughout); pairing and remote-action copy SHALL require explicit user action per machine (passive invariant, D-08 pattern).
**Performance budget**: Docs only; §11 ≤220 lines, §12.1 delta ≤120 lines; normative budgets (fleet freshness, offline queue) labelled as targets for E7 proof.
**Quality gates**: No Fleet sentence promises credential movement; no onboarding step requires typing a path; maintainer approves.
**Evidence/validation proof**: `grep -ni "post-1.0\|schema-only" docs/SPEC.md` shows hits only in history/superseded notes; PR diff stored; pass = Req.1-3 present.
**Visual testing**: Render §§11/12.1 at 100-col/GitHub 1280×800; capture flow list and Fleet contract table; pass = ≤4 cols, all six flows named.
**Files to touch**: `docs/SPEC.md` (§§11, 12.1), `docs/review/pass2/03-smart-onboarding-and-provider-setup.md` (cite only).
**Research basis**: decision-log D-13; pass2/08 §3.1 items 5-6 as corrected by D-13; E7 epic.
**Handover note**: Pass2/08 roadmap deferred Fleet — that sequencing is superseded by D-13 and must not be copied. E7 owns build tasks; §11 owns the contract.

### AJ-E906 PROPOSAL thesis + EPICS resequence rewrite | Status: DONE-EVIDENCED (zcode-primary, 2026-10-13) | Priority P0 | Effort M | Depends: AJ-E901, AJ-E902, AJ-E903, AJ-E904, AJ-E905
**Goal**: Align PROPOSAL (why) and EPICS (order) with Switch → See → Clean and the Stage gates, so no plan document contradicts the decision log.
**User story**: As a contributor choosing work, I want PROPOSAL and EPICS to give the same order as the decision log so I never build demoted scope first.
**Requirements**:
1. PROPOSAL §§1, 6, 9 SHALL adopt the revised thesis and positioning (D-01/D-09), rename to Provider/Account/Instance language, retitle handover as unlimited bidirectional Switching, and add post-close process ownership as a named capability.
2. EPICS SHALL be resequenced: Phase 0 truth/blockers, See (indicators+inspector), Switch (pack, switch-back), Clean (ledger+Cleanup Centre), Fleet at launch integrated per D-13 (not a deferred phase), with the slip rule (cut widening/demoted scope before fixtures) stated.
3. EPICS gates SHALL gain switch-budget evidence, cleanup-collateral evidence and fidelity-register checks (AJ-E807/E808), and demoted items (review queue product, templates library, present mode) SHALL be marked deferred with revisit triggers.
4. Done checklist SHALL enumerate every milestone whose scope text changed and every gate whose evidence list changed.
**UI/UX reasoning**: Contributor UX = NN-g consistency across planning docs; one sequencing vocabulary (Phase names identical in PROPOSAL, EPICS, SPEC and gate files).
**Performance budget**: Docs only; PROPOSAL delta ≤150 lines, EPICS delta ≤250 lines; no runtime claim.
**Quality gates**: No document ranks quota as headline thesis; no Fleet deferral language remains; maintainer approves both diffs in one PR.
**Evidence/validation proof**: Cross-grep for `quota` headline phrasing and Fleet timing in PROPOSAL/EPICS stored in PR; pass = zero contradictions with decision-log D-01/D-13.
**Visual testing**: Render both docs at 100-col/GitHub 1280×800; capture EPICS phase table; pass = ≤4-col tables, phase names match SPEC § references.
**Files to touch**: `docs/PROPOSAL.md`, `docs/plan/EPICS.md`.
**Research basis**: decision-log D-01/D-09/D-13/D-15; pass2/08 §§3.2-3.3, §4 as corrected by D-13.
**Handover note**: Keep milestone IDs for tracking continuity (pass2/08 §4); change scope text, not history. CHANGELOG entries are never rewritten.

### AJ-E907 ADR bundle 0008-0013 drafting and acceptance | Status: DONE-EVIDENCED (zcode-primary, 2026-10-13) | Priority P0 | Effort M | Depends: AJ-E901
**Goal**: Record each load-bearing decision as an ADR so future agents cite decisions instead of re-arguing them.
**User story**: As a new agent taking over cold, I want one ADR per decision with alternatives and consequences, so I can move in minutes.
**Requirements**:
1. ADRs SHALL be: 0008 terminology (E1 AJ-E101 executes the file; this task tracks acceptance), 0009 Portable Context Pack v2, 0010 Spawn ledger and process ownership (incl. D-08 tiers), 0011 Switch Graph and lineage, 0012 Two-clock quota model, 0013 Fleet at launch (identity/transport/security, D-13).
2. Each ADR SHALL follow `docs/adr/0000-template.md`, state Status/Date/Deciders/Context/Decision/Alternatives/Consequences, and name the decision-log ID it implements.
3. Alternatives SHALL include the rejected options from the discussion (e.g. copy-only rename, always-withhold redaction superseded by D-14 user choice) with the rejection reason.
4. The ADR index SHALL list all six with one-line decisions; no ADR SHALL claim implementation that has not happened (Status: Proposed until maintainer accepts).
**UI/UX reasoning**: Reader UX: decision-first headings, ≤250 lines each, tables ≤4 columns; glossary terms match ADR-0008 verbatim across the bundle.
**Performance budget**: Docs only; bundle ≤1,300 lines total; review ≤30 min per ADR.
**Quality gates**: Maintainer sets each ADR to Accepted or records amendments; unresolved ADRs block the SPEC sections that cite them (AJ-E901-E905).
**Evidence/validation proof**: `ls docs/adr/000[89]*.md docs/adr/001[0-3]*.md` + index grep stored in PR; pass = six files, template sections complete, decision-log cross-links resolve.
**Visual testing**: Render each ADR at 100-col; pass = no >4-col table, headings sequential, links live.
**Files to touch**: `docs/adr/0008-*.md` through `docs/adr/0013-*.md` (new), `docs/adr/README.md`.
**Research basis**: decision-log D-02..D-14; pass2/08 §3.4 (0008-0012) plus D-13 (0013 Fleet).
**Handover note**: Numbering is fixed: do not revive pass2/01's ADR-0014 idea. If a decision changes, supersede with a new ADR; never edit an Accepted ADR's Decision section.

### AJ-E908 README truth fix + beta-exit criteria document | Status: DONE-EVIDENCED (zcode-primary, 2026-10-13) | Priority P1 | Effort S | Depends: AJ-E906
**Goal**: Make the public front door and the beta-exit bar say only what is true today.
**User story**: As a new user or contributor, I want the README to match the shipped app so my first hour is not spent reconciling contradictions.
**Requirements**:
1. README SHALL replace stale phase claims with current state (engine, Claude adapter, onboarding, handover v0 labelled beta fidelity), the D-09 tagline+subtitle, and Providers/Accounts/Instances vocabulary.
2. A beta-exit criteria doc SHALL list: fidelity matrix thresholds (AJ-E808), dogfood thresholds (AJ-E809), zero false cleanup kills, Fleet security review, a11y/perf gates (E8), and unsigned-gate closure (AJ-E807).
3. Every capability sentence SHALL be traceable to code, a merged PR, or a labelled-beta decision; aspirational features SHALL be marked planned with epic links.
4. Done checklist SHALL list each corrected README claim with its source.
**UI/UX reasoning**: NN-g #4 consistency + trust: first-screen copy is product UX; honesty rules (D-06) apply to marketing text exactly as to UI fields.
**Performance budget**: Docs only; README delta ≤80 lines; criteria doc ≤150 lines.
**Quality gates**: `grep -ni lossless README.md` justified or removed; no unlabelled cross-Provider claim; maintainer approves.
**Evidence/validation proof**: Claim-by-claim source table in PR; pass = every Req.1 claim sourced, criteria doc linked from EPICS gates.
**Visual testing**: Render README on GitHub preview 1280×800 and 390px width; capture hero section; pass = tagline/subtitle correct, no overflow, links live.
**Files to touch**: `README.md`, `docs/plan/EPICS.md` (beta-exit link), new `docs/qa/beta-exit.md` or gate file (pick one in PR).
**Research basis**: decision-log D-04/D-09/D-11; first-report executive item on README/CHANGELOG contradiction; QUALITY.md truth rule.
**Handover note**: CHANGELOG is history and is never edited to match; fix the README's present-tense claims instead.

### AJ-E909 Provider billing/renewal API research log (Stage-1 record) | Status: DONE-EVIDENCED (zcode-primary, 2026-10-13) | Priority P1 | Effort M | Depends: —
**Goal**: Execute the D-12 mandated research and record, per Provider, whether a sanctioned billing/renewal source exists.
**User story**: As a maintainer, I want the expiry feature built on proven sources so we never guess a billing date.
**Requirements**:
1. For each Provider (Claude/Anthropic, Codex/OpenAI, ZCode, Gemini, plus successors on arrival) the log SHALL record: APIs/SDKs/docs checked (URLs, dates), fields available (plan, renewal, expiry, quota windows), auth required, and whether use is sanctioned for a local supervisor app.
2. Where a source exists, the log SHALL specify a live verification step (sacrificial account, read-only) and its result, or mark the cell UNPROVEN with the blocking action named.
3. Where none is found, the log SHALL include the full search record (queries, sources) per AJ-E810 so "none found" is auditable negative evidence, worded never as vendor denial.
4. The log SHALL conclude with the product rule per Provider: auto-populate (proven), user-entered only (default per D-12), or hidden; E6 tasks consume this as input.
**UI/UX reasoning**: Honest-fields (D-06) for research output: each Provider row shows source state in the same vocabulary the UI will use (Measured/User-entered/Unknown).
**Performance budget**: Research ≤2 days total; log ≤300 lines; re-verification quarterly or on Provider release (AJ-E810 staleness rule).
**Quality gates**: Every Provider has a row; every claim has URL+date+confidence label; no credential used in any test.
**Evidence/validation proof**: The log itself, stored at `docs/research/billing-apis.md` with search records in `docs/qa/evidence/AJ-E909/`; pass = maintainer accepts the per-Provider rule table.
**Visual testing**: Render log at 100-col/GitHub 1280×800; pass = ≤4-col summary table, confidence labels visible as text.
**Files to touch**: `docs/research/billing-apis.md` (new), `docs/tasks/E6-quota-cost-and-billing-research.md` (cross-link), `docs/SPEC.md` §6 (cite resulting rule).
**Research basis**: decision-log D-12 mandatory research item; AJ-E810 evidence standard.
**Handover note**: This is Stage-1-compatible (documents/research only). Live API tests needing paid actions stop at a named maintainer action; never use a real user account.

### AJ-E910 Cross-doc consistency proof and Stage-1 acceptance packet | Status: DONE-EVIDENCED (zcode-primary, 2026-10-13) | Priority P0 | Effort S | Depends: AJ-E901, AJ-E902, AJ-E903, AJ-E904, AJ-E905, AJ-E906, AJ-E907, AJ-E908, AJ-E909
**Goal**: Prove the rewritten plan documents agree with each other and with the decision log, and package Stage 1 for explicit maintainer acceptance.
**User story**: As a maintainer, I want one acceptance packet showing the docs no longer contradict each other, so I can mark the decision log Final with confidence.
**Requirements**:
1. A link/term check SHALL run over SPEC, PROPOSAL, EPICS, ADR-0008..0013, README and the decision log: internal links resolve; Provider/Account/Instance used per ADR-0008; banned usages (`harness` as user noun, `Profile` as rail noun, unlabelled `lossless`) appear only in prohibition/history contexts, each hit listed.
2. A decision-traceability table SHALL map D-01..D-15 to the exact document sections implementing them; any decision without a section SHALL block acceptance.
3. The packet SHALL state the Stage-1 done checklist (D-15): documents rewritten, ADRs drafted, billing research recorded — and explicitly that no code was changed (`git diff --stat` docs-only proof).
4. On maintainer acceptance, the decision log status SHALL flip Draft → Final in the same PR; rejection SHALL record amendments as new unresolved items instead of editing settled text silently.
**UI/UX reasoning**: Governance UX per NN-g visibility: one packet, one checklist, one accept action; no acceptance by silence or by merge alone.
**Performance budget**: Checks <5 min scripted/manual; packet ≤200 lines.
**Quality gates**: Any broken link, banned-term hit without justification, or untraced decision = FAIL; packet cannot pass itself — maintainer sign-off is the gate.
**Evidence/validation proof**: Check outputs + traceability table stored in the acceptance PR and `docs/qa/evidence/AJ-E910/`; pass = all checks green and maintainer acceptance recorded with date/SHA.
**Visual testing**: Render packet at 100-col/GitHub 1280×800; capture traceability table; pass = ≤4-col table, D-01..D-15 all visible, status wording unambiguous.
**Files to touch**: `docs/review/decision-log.md` (status flip on acceptance), SPEC/PROPOSAL/EPICS/ADRs/README (fixes only), `docs/qa/evidence/AJ-E910/`.
**Research basis**: decision-log D-15 done-definition; docs/plan/QUALITY.md gate template; AJ-E801 evidence contract.
**Handover note**: This task closes Stage 1 only. Code implementation stages return for separate approval per D-15; do not let a Final decision log be read as build authorisation.
