# Evidence manifest — AJ-E2024

Task: `docs/tasks/E20-engineering-and-security.md` AJ-E2024, numbers the window can't hold (P1). Found by AJ-E1612's dry run 2 (the rail kept 12 needs while the bell said 9: a 64-bit version rounded to the same float).
Status: **DONE-EVIDENCED**, 2026-11-18, claude-sonnet. Commands re-run on this tree today.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1: the lint scans every ts-rs type's fields, every Tauri command's arguments and result, and the generated TypeScript for `bigint` | `python3 -I scripts/js_number_lint.py` | exit 0 with every crossing judged | exit 0; 2ffdedf4c's inventory: 376 crossings (341 ts-rs fields, 35 command arguments and results), all below 2^53 | PASS | 2ffdedf4c | 2026-11-18 | claude-sonnet |
| R2: each crossing is listed in `scripts/js_number.allow` with why; a left-over entry fails | the same run; `scripts/js_number.allow` (386 lines) | no unlisted crossing, no stale entry | passes; the selftest holds the stale-entry case | PASS | 2ffdedf4c | 2026-11-18 | claude-sonnet |
| R3: a code-bounded value names its function, which must keep masking with `(1 << 53) - 1` | `NoticeSummaryInfo.version` names `centre.rs::version_of` in the allow file; with centre.rs from 045c97acb^ the check fails (recorded in 2ffdedf4c's message) | fails without the mask | passes with the mask (045c97acb) | PASS | 2ffdedf4c, 045c97acb | 2026-11-18 | claude-sonnet |
| R4: the check runs in `just verify-static` and CI, and its selftest fails on a stale entry, an unlisted new 64-bit field, a `bigint` and the pre-fix `version_of` | `python3 -I scripts/js_number_lint.py --selftest`; `justfile` lines 35-36; `.github/workflows/ci.yml` lines 203-204 | selftest passes; both wired | `js_number_lint selftest: ok`; both wired | PASS | 2ffdedf4c | 2026-11-18 | claude-sonnet |
| `AccountInfo.imported_at` is typed as a number, as the window gets it | `ui/src/types/generated/AccountInfo.ts` | a number, not a bigint | `ts(type = "number | null")` | PASS | 94289f074 | 2026-11-18 | claude-sonnet |

## Redaction checklist

- [x] No credentials, tokens or keys
- [x] No transcripts or real names
