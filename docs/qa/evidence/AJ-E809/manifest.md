# Evidence manifest — AJ-E809

Task: `docs/tasks/E8-validation-visual-performance-quality.md` AJ-E809
(beta dogfood programme). 2026-10-19, reviewer claude-opus.

The programme is ready to run; no run has happened yet. It needs real
participants on their own machines and accounts: the maintainer's work,
never an agent's. Kit: [`programme.md`](programme.md) (cohort, consent,
the five scripted tasks, thresholds, cadence), [`run-sheet.md`](run-sheet.md),
[`runs.csv`](runs.csv) (empty) and [`report.md`](report.md) (generated
by `scripts/dogfood_report.py`).

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Req.1: three personas, ≥ 5 active users, 4 weeks | Req.1 | Cohort plan in `programme.md`. The report counts personas covered and weeks with ≥ 5 active users. Today: none | PENDING (maintainer) | 2026-10-19 | claude-opus |
| Req.2: scripted tasks (quota-wall switch, switch-back after reset, zombie cleanup, labelled cross-Provider switch, fleet join) with completion, time, errors and verbatim friction per run | Req.2 | Each task has a trigger and success condition (`programme.md`); one row per run (`run-sheet.md`) | PASS (kit) | 2026-10-19 | claude-opus |
| Req.3: ≥ 80 % unaided completion, ≥ 80 % A→B→A continuity, zero false kills, median switch < 2 min | Req.3 | `scripts/dogfood_report.py` computes each one and exits 1 until all are met. `--selftest`: a passing cohort is MET; one false kill, one unfiled note, or three weeks instead of four is NOT MET ([`selftest.txt`](selftest.txt)) | PASS (tooling) · thresholds PENDING | 2026-10-19 | claude-opus |
| Req.4: findings filed as tasks within 48 h; `[inferred]` proposals wait for dogfood | Req.4 | The report lists every friction note without a `filed_task` and stays NOT MET while any remain. The rule for inferred proposals is in `programme.md` | PASS (kit) | 2026-10-19 | claude-opus |
| Req.5: no credentials or unredacted transcripts; consent and redaction checklist | Req.5 | Consent list in `programme.md` (pseudonymous IDs, redacted friction, consented and blurred captures named by task, deletion on request). `just evidence` scans the store for credential patterns | PASS (kit) | 2026-10-19 | claude-opus |
| Evidence: weekly reports and run sheets; thresholds measured and reported, failures included, follow-ups linked | evidence | Starts with the first week | PENDING (maintainer) | 2026-10-19 | claude-opus |

## Status

IN-PROGRESS. The kit is evidenced; the four-week programme is the
maintainer's to run.
