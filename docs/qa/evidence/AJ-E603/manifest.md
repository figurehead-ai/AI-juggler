# Evidence manifest — AJ-E603

Task: `docs/tasks/E6-quota-cost-and-billing-research.md` AJ-E603.
Branch: `stage2/e602-e604-batch` (docs-only batch — zero crates/ui
changes; `git diff --stat crates/ ui/` empty). Evidence packet:
`docs/research/evidence/billing/`. 2026-10-15, reviewer zcode-primary.

Every capture in the packet uses the AJ-E601 §2 template (binary
version, OS, dated field paths, redacted excerpt, read-only
classification, grade, verification, data class); the per-claim
verdicts below summarise it.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Live captures on the real install(s), version recorded | §2 template complete | Codex Desktop rollouts (cli_version 0.162.0-alpha.17.2 in session_meta) + npm CLI 0.163.0 for the exec --json envelope (throwaway CODEX_HOME, no credentials) | PASS | 2026-10-15 | zcode-primary |
| Every field AJ-E601-graded with citation | grades + capture/doc citations per field | rollout token_usage_record verified (2 sessions >=24h); exec --json envelope verified live; the percentage feed (undocumented rateLimits endpoint) prohibited; renewal negative; tier unverified recorded | PASS | 2026-10-15 | zcode-primary |
| Renewal negative, ≥4 searched surfaces | negatives listed, not omitted | CLI surface, config keys, API docs, CODEX_HOME state, plan docs; auth.json never opened (prohibited) | PASS | 2026-10-15 | zcode-primary |
| Product ceiling fixed for AJ-E607/AJ-E610 | per-field ceiling table | tokens Measured, plan % Estimate + approx resets (styled-as-exact = violation), API-key cost never seat evidence, renewal unknown | PASS | 2026-10-15 | zcode-primary |
| Credential hygiene | grep gate zero | zero credential-marker hits across the evidence folder | PASS | 2026-10-15 | zcode-primary |
| Append-only research addendum | no rewrites | the provider research doc's addendum appended (no earlier text changed) | PASS | 2026-10-15 | zcode-primary |

## Implementation-mapping

- Evidence packet: `docs/research/evidence/billing/` (see the
  provider captures file); append-only addendum in the provider
  research doc; no product code (the task law — findings hand to
  AJ-E607/AJ-E610 as graded packets).
