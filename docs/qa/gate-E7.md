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
| AJ-E703 transport (mTLS, protobuf, replay, limits) | DONE-EVIDENCED | [manifest](evidence/AJ-E703/manifest.md) |
| AJ-E704 discovery + pairing wizard (mDNS, SPAKE2 code, lockouts, attempt log) | DONE-EVIDENCED | [manifest](evidence/AJ-E704/manifest.md) |
| AJ-E705 … AJ-E711 | TODO | — |

## Security negatives (STRIDE rows → tests)

| Negative | Test | State |
|---|---|---|
| Registry tamper rejected | `juggler-link registry::tests::any_tamper_fails_closed` | ✅ green |
| Registry rollback / deletion rejected | `registry::tests::a_rolled_back_or_deleted_registry_fails_closed`, `juggler-app --test fleet_identity` | ✅ green |
| Key file readable by others refused | `identity::tests::a_key_file_readable_by_others_is_refused` | ✅ green |
| Private key never in DB/settings | `juggler-app --test fleet_identity :: the_private_key_never_appears_in_the_database_or_settings` | ✅ green |
| Wrong certificate rejected | `juggler-link --test transport :: a_node_presenting_an_unpinned_key_is_refused_at_tls` | ✅ green |
| TLS 1.2 rejected | `transport :: tls_1_2_is_refused` | ✅ green |
| Unpaired key / machine-id mismatch rejected | `transport :: an_unpaired_console_and_a_machine_id_mismatch_are_refused` | ✅ green |
| Replay / reorder / expiry / wrong console / unmeasured clock | `transport :: replay_reorder_expiry_wrong_console_and_unmeasured_clock_are_rejected` | ✅ green |
| View-only commands rejected node-side | `transport :: view_only_peers_get_telemetry_but_no_commands` | ✅ green |
| Revocation applies to open streams | `transport :: unpairing_revokes_an_open_command_stream_at_the_next_command` | ✅ green |
| Downgrade keeps every check | `transport :: the_mixed_version_matrix` | ✅ green |
| Pre-auth flood bounded; oversize refused; rate limit | `transport :: stalled_handshakes_are_capped_before_any_tls_work`, `command_floods_are_rate_limited_and_oversize_messages_refused` | ✅ green |
| No credential-shaped field on the wire | `proto_contract :: no_field_can_carry_a_secret_or_a_transcript` | ✅ green |
| Breaking protocol change caught | `proto_contract :: no_field_was_removed_or_renumbered` (seeded renumber fails) | ✅ green |
| Wrong pairing code aborts, rotates, pins nothing | `juggler-app --test fleet_link :: a_wrong_code_aborts_rotates_the_code_and_pins_nothing`; Playwright `a mismatched code can't complete the wizard and pins nothing` | ✅ green |
| Pairing brute force locked out (persisted across restart) | `fleet_link :: five_wrong_codes_lock_pairing_and_the_lock_survives_a_restart`; guard unit tests (daily disable) | ✅ green |
| SPAKE2 transcript bound to keys, ids, version, TLS exporter | `juggler-link --test pairing` (7) | ✅ green |
| Pairing link fingerprint mismatch stops the join | `pairing :: a_pairing_link_with_another_fingerprint_is_refused_before_any_guess` | ✅ green |
| Attempt log tamper-evident, never holds the code | `fleet_link` (chain verified; digits absent) | ✅ green |
| Discovery never names the machine; Link off by default | `discovery::tests`, `fleet_link :: link_is_off_by_default_and_pairing_needs_it_on` | ✅ green |
| All others (free-form launch, logs, pack fetch, cleanup, offline sync, account substitution) | AJ-E705 … AJ-E709, AJ-E1206 | pending |

## Dogfood (to record at AJ-E710)

- [ ] AJ-E704: pair two physical Macs on one LAN (screencast + timing,
      target < 60 s); repeat over Tailscale by address.
