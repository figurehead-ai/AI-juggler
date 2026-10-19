# Evidence manifest — AJ-E1101

Task: `docs/tasks/E11-usage-statistics.md` AJ-E1101 (decision D-19,
ADR-0015, document amendments, the schema copy). 2026-10-18, reviewer
claude-opus.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| D-19 in the decision log; ADR-0015 records the architecture | Req.1 | `docs/review/decision-log.md` § D-19 (the twelve points, in our words); `docs/adr/0015-usage-statistics.md`; ADR index updated | PASS | 2026-10-18 | claude-opus |
| SPEC §1, SPEC §8, SECURITY and README describe usage statistics as opt-in, on-device, identifier-free, documented | Req.2 | SPEC §1 Privacy row (adds the opt-in weekly upload, config fetch and crash reports; "with no account and usage statistics off, zero calls to figurehead.ai"); SPEC §8 gains a usage-statistics bullet (endpoints, never-both, overrides, never collected); SECURITY design commitments; README principles line. All link `docs/usage-stats.md`, generated from the schema | PASS | 2026-10-18 | claude-opus |
| `stats/schema.v1.json` byte-identical to the server's copy; a local diff script; the release checklist runs it | Req.3 | `cmp` identical; sha256 `ab86fc0c4f2624d422be808114eaaa564891ab15e1c469b953d54e936a63db2f`. `scripts/stats_schema_diff.sh` → "identical". `docs/RELEASING.md` § Each release step 2 | PASS | 2026-10-18 | claude-opus |
| Terminology: "usage statistics", never "telemetry" | quality gate | `terminology-lint: OK`; E11 and D-19 note that "telemetry" keeps its E7 meaning | PASS | 2026-10-18 | claude-opus |

Also landed here, ahead of AJ-E1106 because the new links need it:
`scripts/gen_usage_stats_doc.py` generates `docs/usage-stats.md` (all
77 metrics) from the schema, and `just verify` and CI fail if the
document drifts.
