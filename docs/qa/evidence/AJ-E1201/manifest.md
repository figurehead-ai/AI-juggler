# Evidence manifest — AJ-E1201

Task: `docs/tasks/E12-optional-account.md` AJ-E1201. 2026-10-18,
reviewer claude-opus.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| D-18 recorded as SETTLED with the maintainer's words and the five points | Req.1 | `docs/review/decision-log.md` D-18; the "Unresolved" note names D-18 | PASS | 2026-10-18 | claude-opus |
| ADR-0014 records the architecture; ADR-0004's relay rejection marked partly superseded | Req.2 | `docs/adr/0014-optional-figurehead-account.md`; ADR index row; ADR-0004 alternatives line annotated | PASS | 2026-10-18 | claude-opus |
| README / PROPOSAL / SECURITY / SPEC §1, §8, §11, §12.1 state the two-path rule | Req.3 | README principle, PROPOSAL guardrail 6, SECURITY design commitment + scope, SPEC privacy row, §8 Keychain items + allowed endpoints, §11.1 account discovery, §12.1 optional onboarding step; `grep -rn "No cloud, no account"` over docs, README.md, SECURITY.md → only the quality-gate line quoting it and this row (no stale rule remains; the frozen review reports keep their historical wording) | PASS | 2026-10-18 | claude-opus |
| Epic E12 added to the catalog and task README | Req.4 | `docs/tasks/E12-optional-account.md` (AJ-E1201..E1207, full task format); TASK-CATALOG total 114 = `grep -h '^### AJ-' docs/tasks/E*.md \| wc -l` | PASS | 2026-10-18 | claude-opus |
| Gates | quality gates | `./scripts/evidence_check.sh` and `./scripts/terminology_lint.sh` OK | PASS | 2026-10-18 | claude-opus |

No server internals are recorded in this repository: the tasks quote
only the public endpoints and request/response shapes the app uses.
