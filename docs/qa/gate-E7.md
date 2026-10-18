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
| AJ-E705 remote inspector (same six tabs, source + freshness, permission-gated actions) | DONE-EVIDENCED | [manifest](evidence/AJ-E705/manifest.md) |
| AJ-E709 indicators, permissions changed only on the node, "Ask each time" prompt, alert routing | DONE-EVIDENCED (Req.2 partial, Req.6 N/A) | [manifest](evidence/AJ-E709/manifest.md) |
| AJ-E711 remote Focus window (executed on the owning Mac, honest outcomes, locked → reported) | DONE-EVIDENCED | [manifest](evidence/AJ-E711/manifest.md) |
| AJ-E706, AJ-E707, AJ-E708, AJ-E710 | TODO | — |

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
| View-only console can't stop a remote Instance (console + node) | `fleet_link :: a_console_sees_the_other_macs_instances_live_and_its_commands_go_through_the_node`, `transport :: view_only_peers_get_telemetry_but_no_commands` | ✅ green |
| Link off drops every connection (no lingering stream) | `transport :: stopping_a_node_never_waits_on_an_open_telemetry_stream` | ✅ green |
| "Ask each time" command never runs without a local Allow | `fleet_link :: ask_each_time_holds_a_command_until_this_macs_user_decides` | ✅ green |
| No link call can change a permission (console can't raise its own access) | no such RPC in `link.proto`; `fleet_set_permission` is a local Tauri command only | ✅ by construction |
| Remote focus never runs on a locked screen; terminal Instances refused; audited both sides | `fleet_link :: a_remote_focus_runs_on_the_owning_mac_and_reports_its_honest_outcome` | ✅ green |
| All others (free-form launch, logs, pack fetch, cleanup, offline sync, account substitution) | AJ-E706 … AJ-E708, AJ-E1206 | pending |

## Dogfood (to record at AJ-E710)

- [ ] AJ-E704: pair two physical Macs on one LAN (screencast + timing,
      target < 60 s); repeat over Tailscale by address.
- [ ] AJ-E711: from the laptop, focus a desktop agent's window on the
      studio Mac (screen recording on the target); repeat locked.
- [ ] AJ-E705: select a remote Instance, read Overview/Processes, time
      the render; Stop it from the console with View + control.
