# Stage 1 acceptance packet (AJ-E910, D-15)

**Claim:** the plan documents now agree with each other and with the
decision log; no product code changed.

## 1. Scope proof (docs-only)

`git diff --stat` (this PR): `docs/*`, `README.md`, task statuses,
evidence store. Zero files under `crates/`, `ui/`, `fuzz/`, CI configs.
Runtime behaviour unchanged — `just verify` (29 suites) and
`pnpm --dir ui build` green before and after.

## 2. Decision traceability D-01..D-15

| Decision | Implemented in |
|---|---|
| D-01 Switch→See→Clean | PROPOSAL summary/pillars; EPICS phases; SPEC §6 ("trigger, not headline") |
| D-02 rename | SPEC §3 vocabulary + §3.5 migration posture; ADR-0008; EPICS Phase A/B |
| D-03 hybrid door | SPEC §7.3; ADR-0011 |
| D-04 beta + validation | SPEC §7.1 badges; beta-exit.md gates 1-2 |
| D-05 switch-back | SPEC §7.4; ADR-0011 |
| D-06 inspector | SPEC §9.2 six tabs × five selections; honest-fields rule |
| D-07 quality signals | SPEC §9.2 Quality tab; "no composite score" normative |
| D-08 cleanup tiers | SPEC §1 amendment, §3.2, §5.2; ADR-0010; §9.4 |
| D-09 tagline | README hero; PROPOSAL summary; SPEC §9.5 |
| D-10 Profile demotion | SPEC §3 vocabulary; ADR-0008 §5 |
| D-11 beta labels | SPEC §7.1; README honesty notes |
| D-12 two clocks | SPEC §6; ADR-0012; billing-apis.md rule table |
| D-13 Fleet at launch | SPEC §11 head; ADR-0013; EPICS phase frame |
| D-14 pack policy | SPEC §1 amendment, §3.3; ADR-0009 |
| D-15 staged scope | This packet; log marked Final on maintainer instruction 2026-10-13 |

No decision lacks a section.

## 3. Term/link checks

- Banned usages appear only in prohibition/history contexts; every hit
  listed and justified in `checks.md` (lossless ×5 justified: 2
  prohibition, 1 same-Provider proven case, 1 checklist, 1 parser-IR
  engineering; post-1.0 ×2 = platform timing, not Fleet).
- Internal links: SPEC↔ADR↔tasks↔beta-exit created this stage resolve
  (spot-checked in review; full link-lint lands with AJ-E801 tooling).

## 4. Stage-1 done checklist (D-15)

- [x] SPEC §§1,3,5,6,7,9,11,12.1 rewritten to the decisions
- [x] PROPOSAL thesis/goals/pillars re-centered
- [x] EPICS resequenced (phase frame + slip rule + gate evidence)
- [x] ADR-0008..0013 drafted and accepted
- [x] README truth-fixed + beta-exit criteria doc
- [x] Billing-API research recorded (docs/research/billing-apis.md)
- [x] Decision log marked Final on explicit maintainer instruction
- [x] No code changed (scope proof above)

## 5. What acceptance authorises

Marking Stage 1 accepted authorises **Stage 2 (implementation)** to
begin per the execution order in IMPLEMENTATION-AGENT-PROMPT §6
(E8 evidence machinery → E1 naming → E4/E2 → E5 → E3 …), each later
stage returning for its own approval per D-15. Wireframe approvals
(docs/wireframes) remain a separate, pending maintainer action required
before UI-building tasks start.
