# Gate E7 — Fleet beta (AJ-E710)

The Fleet beta gate (AJ-E710) fills this file as each E7 task lands:
per-requirement pass/fail with the CI run, dogfood recordings, timings,
the security-negative checklist (one test per STRIDE row of the
ADR-0013 architecture contract), and the list of unvalidated
combinations. Fleet does not exit internal alpha until every P0
security negative below is green.

## Status by task

| Task | State | Evidence |
|---|---|---|
| AJ-E701 architecture contract + threat model | DONE-EVIDENCED (second-reviewer SIGN-OFF) | [manifest](evidence/AJ-E701/manifest.md) |
| AJ-E702 identity + signed trust store | DONE-EVIDENCED | [manifest](evidence/AJ-E702/manifest.md) |
| AJ-E703 … AJ-E711 | TODO | — |

## Security negatives (STRIDE rows → tests)

| Negative | Test | State |
|---|---|---|
| Registry tamper rejected | `juggler-link registry::tests::any_tamper_fails_closed` | ✅ green |
| Registry rollback / deletion rejected | `registry::tests::a_rolled_back_or_deleted_registry_fails_closed`, `juggler-app --test fleet_identity` | ✅ green |
| Key file readable by others refused | `identity::tests::a_key_file_readable_by_others_is_refused` | ✅ green |
| Private key never in DB/settings | `juggler-app --test fleet_identity :: the_private_key_never_appears_in_the_database_or_settings` | ✅ green |
| All others (pairing, transport, replay, permissions, flood, logs, pack fetch, cleanup, offline) | AJ-E703 … AJ-E709 | pending |
