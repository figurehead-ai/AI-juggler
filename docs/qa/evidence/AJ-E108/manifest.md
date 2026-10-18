# Evidence manifest — AJ-E108 (docs sweep + terminology lint gate)

| Claim | Command | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|---|
| Req.1-2 SPEC/PROPOSAL/EPICS swept | `./scripts/terminology_lint.sh` | OK (0 hits) | glossary box added (ADR-0008 wording, linked from PROPOSAL+EPICS); vendor-compliance sentences untouched; review/** research/** prior ADRs verbatim (out of scope, listed here) | PASS | 2026-10-13 | zcode-primary |
| Req.4 lint gate wired | `just verify` includes terminology; CI verify step added | green | both wired; ≤2 s runtime | PASS | 2026-10-13 | zcode-primary |
| Seeded violation proof | `seeded-violation-red.log` | gate red on violation, green after removal | seeded exit=1 (2 hits), clean exit=0 | PASS | 2026-10-13 | zcode-primary |
| Req.5 CHANGELOG Renamed entry | CHANGELOG.md | present | entry states stored data + wire IDs unchanged; v5 auto-migration | PASS | 2026-10-13 | zcode-primary |
