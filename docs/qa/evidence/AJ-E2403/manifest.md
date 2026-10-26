# Evidence manifest — AJ-E2403

Task: `docs/tasks/E24-processes.md` AJ-E2403, The user's stop of another app's leftover (D-41b). Branch `e24-processes`. Status: **IN-PROGRESS**.

## What exists now

`crates/juggler-app/src/features/processes/stop.rs` (the check, the sheet's data, the audited ladder) and `processes_observed_verify` / `processes_observed_stop`.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.2/3 every leg of the identity check fails closed | `cargo test -p juggler-app --lib processes::stop::tests::the_identity_check_fails_closed_on_every_leg` | 13 refusals | pass | PASS | — | 2026-10-25 | agent |
| Req.4 a verified leftover of an ended session stops, audited as user:observed with the program's name only | `cargo test -p juggler-app --test observed_stop a_verified_leftover_of_an_ended_session_stops_and_is_audited` | exited on TERM; actor and name | pass | PASS | — | 2026-10-25 | agent |
| Req.3 every refusal sends zero signals and audits nothing | `cargo test -p juggler-app --test observed_stop every_refusal_sends_zero_signals` | the fixtures alive | pass | PASS | — | 2026-10-25 | agent |
| Req.1/5 no rule ever stops it (collateral) | `cargo test -p juggler-app --test observed_stop no_automatic_rule_ever_stops_another_apps_leftover` | alive after the sweep past every grace | pass | PASS | — | 2026-10-25 | agent |
| Visual: the sheet for another app's leftover and its refusal | AJ-E2404 gallery | looked at | not built yet | UNPROVEN | — | 2026-10-25 | agent |
| Live: stop a lab leftover of an ended Terminal session | `docs/qa/processes-lab.md` step 7 | as written there | not run yet | UNPROVEN | — | 2026-10-25 | agent |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (none: fixtures only)
