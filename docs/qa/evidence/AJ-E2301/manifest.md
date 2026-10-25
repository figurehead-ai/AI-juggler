# Evidence manifest — AJ-E2301

Task: `docs/tasks/E23-session-runtime.md` AJ-E2301, The runtime lab on real tools. Branch `e23-runtime`. Status: **IN-PROGRESS**: the runnable checklist and the stat-only snapshot are written; the first live run (2026-10-25, cherry-picked as 23985886) proved Req.1 in part. The rest is the coordinator's live run.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.5 runnable checklist, before/after snapshot, cleanup | `docs/qa/runtime-lab.md`; `scripts/qa/runtime_lab_snapshot.sh take|diff` | exact commands; stat only, no contents read | written | PASS | db6317cb | 2026-10-25 | agent |
| Req.1 stdio JSON-RPC, weekly window as primary, an approval declined | `docs/research/session-runtime.md` "Lab, 2026-10-25" | live on a lab account | as recorded | PASS | 23985886 | 2026-10-25 | agent |
| Req.1 two clients on one thread; non-ephemeral resume; `codex resume` after hand-over | docs/qa/runtime-lab.md (the coordinator's live lab) | live | not run yet | UNPROVEN | — | 2026-10-25 | agent |
| Req.2 `--settings` hooks and status line merge with the user's own; PermissionRequest timing | docs/qa/runtime-lab.md (the coordinator's live lab) | both run | not run yet | UNPROVEN | — | 2026-10-25 | agent |
| Req.3 stream-json: an approval, an AskUserQuestion answer, a rate_limit_event | docs/qa/runtime-lab.md (the coordinator's live lab) | live | not run yet (viability shown: apiKeySource none) | UNPROVEN | — | 2026-10-25 | agent |
| Req.4 headless → terminal → headless keeps one transcript | docs/qa/runtime-lab.md (the coordinator's live lab) | one transcript | not run yet | UNPROVEN | — | 2026-10-25 | agent |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (none: fixtures only)
