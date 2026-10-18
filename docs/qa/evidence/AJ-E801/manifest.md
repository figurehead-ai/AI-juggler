# Evidence manifest — AJ-E801 (evidence store + checks)

| Claim / Req ID | Command / fixture | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1-2 store + template | `ls docs/qa/evidence/_template/manifest.md` (6 fields + redaction checklist) | template exists, fields complete | present | PASS | (this PR) | 2026-10-13 | zcode-primary |
| Req.3 gate links resolve | `./scripts/evidence_check.sh` | exit 0 | `evidence-check: OK` | PASS | (this PR) | 2026-10-13 | zcode-primary |
| Req.5 index generated | `./scripts/evidence_index.sh` then check | all task dirs indexed | 10 Stage-1 dirs + new E8 dirs indexed | PASS | (this PR) | 2026-10-13 | zcode-primary |
| Req.6 secret scan | check step 2 over evidence store | 0 patterns | 0 hits | PASS | (this PR) | 2026-10-13 | zcode-primary |
| Gate wiring | `just verify` includes evidence; CI verify job runs it | green locally | `just verify` green (29 suites + evidence OK) | PASS | (this PR) | 2026-10-13 | zcode-primary |

## Notes
- Content-addressing rule (Req.4) is enforced by convention + template;
  the visual harness already appends (determinism artifacts deduped on
  success). Binary re-runs never overwrite cited files.
