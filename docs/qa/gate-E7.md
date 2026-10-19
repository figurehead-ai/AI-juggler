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
| AJ-E707 remote cleanup (node re-verifies, its own ladder, audited both sides, no command lines on the wire) | DONE-EVIDENCED | [manifest](evidence/AJ-E707/manifest.md) |
| AJ-E708 offline and conflicts (durable outbox/replica, sequence-vector catch-up, epochs, skew flag, sync chip, `Diverged` + conflict sheet) | DONE-EVIDENCED (labels/settings N/A — none synced) | [manifest](evidence/AJ-E708/manifest.md) |
| AJ-E706 cross-machine switching (catalog targets, T2 preconditions + T3 offered, click-time push, confirmed path mapping, shared lineage, switch-back) | DONE-EVIDENCED | [manifest](evidence/AJ-E706/manifest.md) |
| AJ-E710 beta gate | TODO | — |

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
| A remote Clean of an unverifiable process sends zero signals | `fleet_link :: a_console_cleans_only_what_the_node_verifies_and_hears_each_outcome` | ✅ green |
| No command line crosses the link | same test (only the executable name), `proto_contract` field allowlist | ✅ green |
| A rogue peer (found by discovery, not pinned) receives zero deltas, even skipping Hello | `transport :: a_rogue_peer_skipping_hello_receives_zero_deltas` | ✅ green |
| Offline catch-up loses nothing and overwrites nothing; replay changes nothing | `juggler-app --test fleet_sync` (4), `juggler-core --test fleet_sync_props` (3 properties) | ✅ green |
| No transcript or pack byte crosses before the user's click; no pull exists | `fleet_switch :: t2_at_the_same_path…` (no handover event before the click), `proto_contract` (no path / cwd / transcript field) | ✅ green |
| A received handover is verified, capped, one at a time; refused when access changed | `transport :: a_corrupt_short_or_oversized_handover_changes_nothing`, `a_view_only_console_gets_no_handover_taken`, `handovers_never_ride_the_command_stream…` | ✅ green |
| T2 never runs on a version mismatch or an unconfirmed path (destination re-checks) | `fleet_switch :: a_version_mismatch…`, `another_checkout_path…`, `the_destination_rechecks…` | ✅ green |
| A diverged original is never overwritten; destructive actions wait for the user's choice | `fleet_switch :: a_diverged_original…`, `keep_mine_…` | ✅ green |
| Spoofed delta origin dropped | `remote::tests::a_delta_naming_another_machine_is_never_applied` | ✅ green |
| All others (free-form launch, logs, account substitution) | AJ-E1206, AJ-E710 log scan | pending |

## AJ-E710 — the gate, requirement by requirement

The automated portion is one command: `just gate-e7` (script
`scripts/gate_e7.sh`). It runs `juggler-link`, `juggler-supervisor`,
`juggler-core` and every Fleet integration suite of the app, prints
pass counts and every measured `[perf]` number, and exits non-zero —
the gate fails closed — if anything is red. Each run is filed under
`docs/qa/evidence/AJ-E710/automated-<date>.md`. The rows below that
need two physical Macs or two testers stay open until the maintainer
runs them; nothing here claims them.

| # | Requirement | State | Evidence |
|---|---|---|---|
| 1a | Pair < 60 s on two physical Macs | PENDING — dogfood | Loopback: `fleet_link :: the_right_code_and_both_confirmations…` (seconds); physical run below |
| 1b | One board across 3 machines (or a filed waiver) | PENDING — dogfood | Board: gallery `46-fleet-board`; 3-machine run below |
| 1c | Cross-machine A→B→A with the continuity checklist | PENDING — dogfood | Loopback: `fleet_switch` (T2 there, one-delta switch-back here, original never overwritten) |
| 1d | Remote orphan cleanup with zero collateral | PENDING — dogfood | Loopback: `fleet_link :: a_console_cleans_only_what_the_node_verifies…` (impostor untouched) |
| 2 | Security checklist — one negative per STRIDE row | **PASS (automated)** | The *Security negatives* table above, all green, including the AJ-E710 additions: no credential or secret reaches the other Mac or any log (`fleet_switch :: no_credential_or_secret…`), forged free-form launches refused (`fleet_switch :: forged_free_form_launches…`), parallel pairing sessions refused (`pairing :: only_one_session_runs_at_a_time`) |
| 3 | Every unvalidated Provider pair / path listed by name; `Unvalidated` at the point of action | **PASS** | List below; the wizard shows `F… · Unvalidated` on every cross-machine destination ([AJ-E708](evidence/AJ-E708/manifest.md), gallery [`50-cross-switch`](evidence/_gallery/50-cross-switch)) |
| 4 | Measured performance vs budgets on the gate hardware | PARTIAL — loopback measured, gate hardware pending | Every run of `just gate-e7` files the numbers (heartbeat RTT, 1,000-delta catch-up, target list, handover throughput, divergence round trip); the two-Mac numbers come from the dogfood |
| 5 | Mixed versions: current ↔ previous minor | PASS (in-process) / N/A (physical) | `transport :: the_mixed_version_matrix` (telemetry works, commands degrade with the banner, nothing crashes). Protocol v1.0 is unreleased, so no previous minor exists to run physically yet |
| 6 | Link off returns the app to fully local behaviour, tested | **PASS** | [AJ-E706](evidence/AJ-E706/manifest.md): `fleet_switch :: link_off_returns_both_macs_to_fully_local_behaviour` (after real fleet use: no destination offered, local switch targets listed, nothing marked diverged) |
| 7 | No pass with a P0 security negative red | **PASS (by construction)** | `scripts/gate_e7.sh` exits non-zero on any red suite; beta labelling waives nothing |
| — | Two tester sign-offs | PENDING | Maintainer |

### Unvalidated combinations (D-04)

Nothing cross-machine is validated yet — no live two-Mac cell exists.
Every one of these ships labelled `Unvalidated` in the wizard:

- Cross-machine T2 (transplant): claude-code → claude-code.
- Cross-machine T3 (replay): every Provider pair the destination has
  installed — today claude-code → claude-code (as replay); codex and
  zcode pairs once their adapters land (see `gate-E2-validation.md`,
  where they are already PENDING locally).
- Path mapping: every mapped (non-identical) project path.
- Remote cleanup, remote focus, remote stop: loopback only.

Local validation status per pair stays in `gate-E2-validation.md`.

## Dogfood (to record at AJ-E710)

- [ ] AJ-E704: pair two physical Macs on one LAN (screencast + timing,
      target < 60 s); repeat over Tailscale by address.
- [ ] AJ-E707: leave a real `vite` orphan on one Mac, clean it from the
      other; keep before/after `lsof` and the audit excerpt.
- [ ] AJ-E711: from the laptop, focus a desktop agent's window on the
      studio Mac (screen recording on the target); repeat locked.
- [ ] AJ-E708: take the laptop offline for a day of work, reconnect;
      keep the node's `sync_catch_up` log lines and the queued-changes
      chip before/after.
- [ ] AJ-E706: laptop → desktop → laptop with the continuity checklist
      and the delta-turn screenshot; a version-mismatch run (T2 blocked,
      T3 offered); a network capture while browsing targets (zero
      transcript bytes); flip the live per-pair cell only if it passes.
- [ ] AJ-E710: three machines on one board (or file the waiver);
      run `just gate-e7` on the gate build and file its report; record
      the timing table on the gate hardware; keyboard-only pair, switch
      and clean; two tester sign-offs.
- [ ] AJ-E705: select a remote Instance, read Overview/Processes, time
      the render; Stop it from the console with View + control.

## Sign-off

Added by AJ-E807: a gate is signed here with the decision, the reviewer,
the date and the commit it approves (`scripts/gate_packet.py signed`
checks it). Until then this gate is open.

| Field | Value |
|---|---|
| Decision | UNSIGNED |
| Reviewer | |
| Date | |
| Commit | |
