# AJ-E903 evidence (Stage 1, docs-only)

- Requirement source: docs/tasks/E9-docs-adrs-and-plan-rewrite.md (AJ-E903)
- Change: SPEC section rewritten per the task's SHALLs (see PR diff).
- Check: `grep -n "Provider\|Instance\|SwitchEdge" docs/SPEC.md` — all
  Req terms present; done-checklist embedded in the section.
- Gates: docs-only; `just verify` green pre/post (29 suites, baseline log
  in AJ-E910); `git diff --stat` shows docs/* + tasks/* only.
- Verdict: PASS (maintainer acceptance recorded via decision-log Final
  + Stage-1 approval, 2026-10-13).
