# Evidence manifest — AJ-E604

Task: `docs/tasks/E6-quota-cost-and-billing-research.md` AJ-E604.
Branch: `stage2/e602-e604-batch` (docs-only batch — zero crates/ui
changes; `git diff --stat crates/ ui/` empty). Evidence packet:
`docs/research/evidence/billing/`. 2026-10-15, reviewer zcode-primary.

Every capture in the packet uses the AJ-E601 §2 template (binary
version, OS, dated field paths, redacted excerpt, read-only
classification, grade, verification, data class); the per-claim
verdicts below summarise it.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Live captures on the real install(s), version recorded | §2 template complete | ZCode desktop 3.14.5 / CLI 0.16.9; the DB CLONED (cp trio) and queried read-only; live-DB mtime movement is the running app's own WAL writes — the probe had no write path | PASS | 2026-10-15 | zcode-primary |
| Every field AJ-E601-graded with citation | grades + capture/doc citations per field | model_usage verified (2 days >=24h); formula/tiers/off-peak documented-sanctioned (docs re-fetched 2026-10-15); bucket-mapping inferred; pool + renewal negative | PASS | 2026-10-15 | zcode-primary |
| Renewal negative, ≥4 searched surfaces | negatives listed, not omitted | devpack docs + FAQ, DB tables, setting.json keys, CLI commands; credentials file never opened; app endpoints not probed (prohibited) | PASS | 2026-10-15 | zcode-primary |
| Product ceiling fixed for AJ-E607/AJ-E610 | per-field ceiling table | burn Computed (the +-0 script ships beside the packet; MCP x1.2 gap recorded), pool Estimate-at-best vs user caps, renewal unknown | PASS | 2026-10-15 | zcode-primary |
| Credential hygiene | grep gate zero | zero credential-marker hits across the evidence folder | PASS | 2026-10-15 | zcode-primary |
| Append-only research addendum | no rewrites | the provider research doc's addendum appended (no earlier text changed) | PASS | 2026-10-15 | zcode-primary |

## Implementation-mapping

- Evidence packet: `docs/research/evidence/billing/` (see the
  provider captures file); append-only addendum in the provider
  research doc; no product code (the task law — findings hand to
  AJ-E607/AJ-E610 as graded packets).
