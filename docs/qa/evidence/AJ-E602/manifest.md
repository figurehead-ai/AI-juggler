# Evidence manifest — AJ-E602

Task: `docs/tasks/E6-quota-cost-and-billing-research.md` AJ-E602.
Branch: `stage2/e602-e604-batch` (docs-only batch — zero crates/ui
changes; `git diff --stat crates/ ui/` empty). Evidence packet:
`docs/research/evidence/billing/`. 2026-10-15, reviewer zcode-primary.

Every capture in the packet uses the AJ-E601 §2 template (binary
version, OS, dated field paths, redacted excerpt, read-only
classification, grade, verification, data class); the per-claim
verdicts below summarise it.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Live captures on the real install(s), version recorded | §2 template complete | claude 2.1.300 direct + the imported managed copies (authenticated-era transcripts — usage + cost-state shapes captured, redacted) | PASS | 2026-10-15 | zcode-primary |
| Every field AJ-E601-graded with citation | grades + capture/doc citations per field | transcripts local-observable verified; cost-state verified; statusline rate_limits candidate (the unauth login screen never invokes the statusline — honest negative); stats-cache candidate (no live instance); renewal negative | PASS | 2026-10-15 | zcode-primary |
| Renewal negative, ≥4 searched surfaces | negatives listed, not omitted | Console/Admin API, CLI surface, statusline schema, local config, + the API-key-vs-subscription gotcha recorded | PASS | 2026-10-15 | zcode-primary |
| Product ceiling fixed for AJ-E607/AJ-E610 | per-field ceiling table | tokens Measured (local scope), cost Computed, quota % Measured only after the authenticated capture verifies it, renewal unknown/user-entered | PASS | 2026-10-15 | zcode-primary |
| Credential hygiene | grep gate zero | zero credential-marker hits across the evidence folder | PASS | 2026-10-15 | zcode-primary |
| Append-only research addendum | no rewrites | the provider research doc's addendum appended (no earlier text changed) | PASS | 2026-10-15 | zcode-primary |

## Implementation-mapping

- Evidence packet: `docs/research/evidence/billing/` (see the
  provider captures file); append-only addendum in the provider
  research doc; no product code (the task law — findings hand to
  AJ-E607/AJ-E610 as graded packets).
