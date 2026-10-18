# Pass 2 — Critic Crosschecks (09)

> Scope: critic only. No new design. Checks parts 01–08 against every
> non-blank line of `docs/expected-worfklow.md`, against each other,
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
mandates Instances (`expected-worfklow.md:11`). 01, 04, 05,
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
