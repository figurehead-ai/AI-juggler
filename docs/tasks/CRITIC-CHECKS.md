# Critic Checks — Audit of This Task System

> Method: programmatic audit over `docs/tasks/E*.md` (task headers and
> template fields parsed, not eyeballed), plus manual cross-epic
> review. Re-run the audit after any epic edit; commands are given so
> a cold agent can reproduce every number here.

## 1. Template compliance (verified by script)

Audit command (paraphrase): split each epic on `^### AJ-`, assert all
11 fields present — Goal, User story, Requirements, UI/UX reasoning,
Performance budget, Quality gates, Evidence/validation proof, Visual
testing, Files to touch, Research basis, Handover note.

| Epic file | Tasks | Incomplete templates |
|---|---|---|
| E1-terminology-and-naming.md | 8 | none |
| E2-switching-engine.md | 10 | none |
| E3-smart-onboarding.md | 9 | none |
| E4-indicators-and-inspector.md | 15 | none |
| E5-process-ownership-and-cleanup.md | 12 | none |
| E6-quota-cost-and-billing-research.md | 11 | none |
| E7-fleet-at-launch.md | 11 | none |
| E8-validation-visual-performance-quality.md | 10 | none |
| E9-docs-adrs-and-plan-rewrite.md | 10 | none |
| E10-derived-workflows-and-beta.md | 11 | none |
| **Total** | **107** | **0** |

Field presence proves structure, not quality of prose; reviewers must
still read the claimed task in full before claiming it.

## 2. Known defects and watch items (honest register)

1. **E8/E9 were completed after a sub-agent quota failure.** Their
   final task counts (10/10) and templates pass the audit, but they
   received less parallel research depth than E1-E7. First executor
   of any E8/E9 task should re-verify cited file:line references
   before building on them.
2. **Visual testing says "None" in a minority of doc/API tasks**
   (e.g. ADR and API-rename tasks). Each such task states why and
   gives a render check instead; that is accepted, but any task that
   grows a UI surface must be amended to full AJ-E802/E803 captures
   before code starts.
3. **Duplicate coverage, canonical owners** (do not build twice):
   - Fidelity truth: AJ-E808 (E8) owns the register; AJ-E210 (E2)
     owns the UI consumption and beta gate wiring.
   - State emission (Stuck/WaitingUser/QuotaBlocked): supervisor is
     the single source; AJ-E405 (E4 read-model) and AJ-E1004 (E10
     workflow) consume it — coordinate, no second state machine.
   - Process/orphan data: AJ-E410 (E4 tab contract) renders what
     E5 (AJ-E501-E506) produces; E5 is canonical for data.
   - Onboarding policy choice: AJ-E309 (E3) owns the first-run UI;
     AJ-E208 (E2) owns the pack policy engine it writes to.
   - Billing research: AJ-E909 (E9, Stage-1 record) and AJ-E601-605
     (E6, deep per-Provider) overlap by design; E909 is the Stage-1
     summary, E6 the execution depth. Keep one rule table (E909) and
     link, never fork conclusions.
   - ADR-0008 file: AJ-E101 (E1) writes it; AJ-E907 (E9) tracks the
     0008-0013 bundle acceptance. Same file, two roles — no duplicate.
4. **Dependency shape**: no cycles were found by manual inspection of
   `Depends:` edges (E9 is the sink for docs; E8 gates consume most
   epics at gate time, not build time). Any new task must re-check
   its edges against TASK-CATALOG.md before work starts.
5. **Effort estimates are T-shirt judgements from research**, not
   measured velocity. Re-estimate at stage approval; do not quote
   them as commitments.
6. **File paths in older research may have drifted.** Every task's
   executor must re-verify its **Files to touch** against the live
   tree first; a moved file updates the task, not the plan's memory.

## 3. Standards every future critic must re-check

- Count check: `grep -h "^### AJ-" docs/tasks/E*.md | wc -l` equals
  the TASK-CATALOG total (107 after AJ-E413-E415/AJ-E711).
- Template check: the 11-field audit above reports zero incomplete.
- Evidence check: no status reads DONE-EVIDENCED without an existing
  `docs/qa/evidence/<task-id>/` path linked from the epic file.
- Decision check: no task text contradicts D-01..D-15; pass2
  "recommended defaults" superseded by the decision log (notably
  redaction D-14, cleanup tiers D-08, fleet D-13) must not re-enter
  through task prose.
- Visual check: every UI task names viewports, states, themes and a
  comparison/a11y method, or explicitly justifies "None".

## 4. What this system deliberately does not contain

- No implementation code, no test code, no release artefacts (D-15).
- No re-decision of D-01..D-15; amendments need a new decision-log
  entry with maintainer approval.
- No claim that any task is done. At generation, all 107 are TODO,
  and the product code was not modified by this breakdown.
