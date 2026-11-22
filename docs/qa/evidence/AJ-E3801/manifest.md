# Evidence manifest — AJ-E3801

Task: AJ-E3801 (the AI-jobs router ADR, E38, D-121). Status: **DONE**: ADR-0034 accepted by the coordinating session on 2026-11-16.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 the seam generalises `SummaryEngine` and says what stays in ADR-0033 and what moves | read [ADR-0034 §1 and §2](../../../adr/0034-ai-jobs-router.md) | both lists present | present | PASS | a814084b3 | 2026-11-16 | Coordinating session |
| R2 three needs, three capabilities, the matching rule and tie-break, and changed engines | read ADR-0034 §2 and §3 | rule, tie-break and change handling written | present | PASS | a814084b3 | 2026-11-16 | Coordinating session |
| R3 a job with no engine is hidden; the UI learns it from one query | read ADR-0034 §3.4 and §9's `a_job_with_no_engine_is_absent_from_the_query_answer` | one query named | present | PASS | a814084b3 | 2026-11-16 | Coordinating session |
| R4 disk rules, the guard, tool use per engine, and where the orchestrator would attach | read ADR-0034 §5, §6, §7 and §8 | each covered | present | PASS | a814084b3 | 2026-11-16 | Coordinating session |
| R5 reviewed by the coordinating session before AJ-E3802 starts | ADR-0034 Status: Accepted, 2026-11-16 | acceptance recorded | accepted | PASS | — | 2026-11-16 | Coordinating session |
